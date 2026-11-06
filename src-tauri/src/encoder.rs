use crate::ffmpeg::{self, CREATE_NO_WINDOW};
use std::ffi::OsString;
use std::io::{self, BufRead, BufReader, Read};
use std::os::windows::io::{AsRawHandle, RawHandle};
use std::os::windows::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

const BELOW_NORMAL_PRIORITY_CLASS: u32 = 0x0000_4000;
const STDERR_LINES: usize = 6;

// Undocumented, as Task Manager's Suspend uses them.
#[link(name = "ntdll", kind = "raw-dylib")]
unsafe extern "system" {
    fn NtSuspendProcess(process: RawHandle) -> i32;
    fn NtResumeProcess(process: RawHandle) -> i32;
}

// The app's one kill-on-close job: Windows kills every encode in it when the app ends, a crash
// included, and when this is dropped. Only ffmpeg joins; a job holding the app itself is inherited by everything it starts,
// the updater's installer too.
pub struct Encoder {
    job: win32job::Job,
}

pub struct Running {
    child: Arc<Mutex<Child>>,
    progress: JoinHandle<()>,
    stderr: JoinHandle<String>,
}

impl Encoder {
    pub fn new() -> io::Result<Encoder> {
        let job = win32job::Job::create().map_err(io::Error::other)?;
        let mut info = job.query_extended_limit_info().map_err(io::Error::other)?;
        info.limit_kill_on_job_close();
        job.set_extended_limit_info(&info)
            .map_err(io::Error::other)?;

        Ok(Encoder { job })
    }

    // Starts ffmpeg; `on_progress` gets the seconds of output written so far, on a thread of its own.
    pub fn start(
        &self,
        args: &[OsString],
        on_progress: impl FnMut(f64) + Send + 'static,
    ) -> io::Result<Running> {
        let mut child = Command::new("ffmpeg")
            .args(args)
            .creation_flags(BELOW_NORMAL_PRIORITY_CLASS | CREATE_NO_WINDOW)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(ffmpeg::not_installed)?;

        if let Err(e) = self.job.assign_process(child.as_raw_handle() as isize) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other(format!(
                "ffmpeg couldn't join the job: {e}"
            )));
        }

        let stdout = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");

        Ok(Running {
            child: Arc::new(Mutex::new(child)),
            progress: thread::spawn(move || read_progress(stdout, on_progress)),
            stderr: thread::spawn(move || last_lines(stderr)),
        })
    }
}

impl Running {
    pub fn pause(&self) -> io::Result<()> {
        self.signal(NtSuspendProcess)
    }

    pub fn resume(&self) -> io::Result<()> {
        self.signal(NtResumeProcess)
    }

    // `wait` then reports a failure.
    pub fn kill(&self) -> io::Result<()> {
        self.child.lock().unwrap().kill()
    }

    // Blocks until ffmpeg ends; an error carries its last stderr lines.
    pub fn wait(self) -> Result<(), String> {
        let _ = self.progress.join();
        let stderr = self.stderr.join().unwrap_or_default();
        let status = self
            .child
            .lock()
            .unwrap()
            .wait()
            .map_err(|e| e.to_string())?;

        match status.success() {
            true => Ok(()),
            false => Err(format!("ffmpeg failed ({status}): {stderr}")),
        }
    }

    fn signal(&self, nt: unsafe extern "system" fn(RawHandle) -> i32) -> io::Result<()> {
        let child = self.child.lock().unwrap();
        // SAFETY: the handle belongs to the child, which the lock keeps alive.
        let status = unsafe { nt(child.as_raw_handle()) };

        match status >= 0 {
            true => Ok(()),
            false => Err(io::Error::other(format!("NTSTATUS {status:#x}"))),
        }
    }
}

fn read_progress(stdout: impl Read, mut on_progress: impl FnMut(f64)) {
    for line in BufReader::new(stdout).lines().map_while(Result::ok) {
        if let Some(seconds) = progress_seconds(&line) {
            on_progress(seconds);
        }
    }
}

fn last_lines(stderr: impl Read) -> String {
    let lines: Vec<String> = BufReader::new(stderr)
        .lines()
        .map_while(Result::ok)
        .collect();
    let from = lines.len().saturating_sub(STDERR_LINES);

    lines[from..].join(" | ")
}

// Seconds of output written, from an `out_time_us=` line; ffmpeg prints N/A or a negative before the first frame.
fn progress_seconds(line: &str) -> Option<f64> {
    let micros: i64 = line.strip_prefix("out_time_us=")?.parse().ok()?;

    (micros >= 0).then(|| micros as f64 / 1e6)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encode_args::{self, Source};
    use crate::ffmpeg::Colour;
    use crate::review::Piece;
    use crate::verify;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    #[test]
    fn reads_the_seconds_written_and_skips_the_rest() {
        assert_eq!(progress_seconds("out_time_us=1500000"), Some(1.5));
        assert_eq!(progress_seconds("out_time_us=N/A"), None);
        assert_eq!(progress_seconds("out_time_us=-9223372036854775807"), None);
        assert_eq!(progress_seconds("out_time_ms=1500000"), None);
        assert_eq!(progress_seconds("progress=continue"), None);
    }

    #[test]
    fn keeps_the_last_stderr_lines() {
        let text = (1..=9).map(|n| format!("line {n}\n")).collect::<String>();
        assert_eq!(
            last_lines(text.as_bytes()),
            "line 4 | line 5 | line 6 | line 7 | line 8 | line 9"
        );
    }

    fn make_source(folder: &Path, seconds: u32, size: &str) -> PathBuf {
        let file = folder.join("source.mp4");
        let video = format!("testsrc2=size={size}:rate=30:duration={seconds}");
        let audio = format!("sine=frequency=440:duration={seconds}");
        let mut ffmpeg = Command::new("ffmpeg");
        ffmpeg
            .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg(video)
            .args(["-f", "lavfi", "-i"])
            .arg(audio)
            .args(["-c:v", "mpeg4", "-pix_fmt", "yuv420p", "-c:a", "aac"])
            .arg(&file);
        let made = ffmpeg::run(ffmpeg).unwrap();
        assert!(
            made.status.success(),
            "{}",
            String::from_utf8_lossy(&made.stderr)
        );

        file
    }

    fn source_of(file: &Path, seconds: f64, width: i64, height: i64) -> Source<'_> {
        Source {
            path: file,
            duration_s: seconds,
            width,
            height,
            fps: 30.0,
            audio_tracks: 1,
            colour: Colour::default(),
        }
    }

    fn encode_and_check(pieces: &[Piece]) {
        let folder = tempfile::tempdir().unwrap();
        let file = make_source(folder.path(), 4, "320x180");
        let streams = ffmpeg::streams(&file).unwrap();
        assert_eq!(streams.audio_tracks, 1);

        let source = source_of(&file, 4.0, 320, 180);
        let pieces = encode_args::normalise(pieces, 4.0);
        let output = folder.path().join("out.mp4");
        let args = encode_args::args(&source, &pieces, 46, &output);
        let seen = Arc::new(AtomicU64::new(0));
        let seen_by_reader = seen.clone();

        let encoder = Encoder::new().unwrap();
        let running = encoder
            .start(&args, move |_| {
                seen_by_reader.fetch_add(1, Ordering::SeqCst);
            })
            .unwrap();
        running.wait().unwrap();
        let kept: f64 = pieces.iter().map(|p| p.end_s - p.start_s).sum();
        verify::probes(&output, encode_args::output_size(320, 180), kept).unwrap();
        verify::decodes(&output).unwrap();
        assert!(seen.load(Ordering::SeqCst) > 0);
    }

    #[test]
    fn encodes_one_piece_to_a_file_that_probes_and_decodes() {
        encode_and_check(&[Piece {
            start_s: 0.5,
            end_s: 2.5,
        }]);
    }

    #[test]
    fn encodes_two_pieces_joined_to_a_file_that_probes_and_decodes() {
        encode_and_check(&[
            Piece {
                start_s: 0.2,
                end_s: 1.2,
            },
            Piece {
                start_s: 2.0,
                end_s: 3.5,
            },
        ]);
    }

    #[test]
    fn a_damaged_output_fails_the_decode_check() {
        let folder = tempfile::tempdir().unwrap();
        let file = make_source(folder.path(), 2, "320x180");
        let mut bytes = std::fs::read(&file).unwrap();
        let from = bytes.len() / 2;
        bytes[from..from + 2000].fill(0xAB);
        std::fs::write(&file, bytes).unwrap();

        assert!(verify::decodes(&file).is_err());
    }

    #[test]
    fn pauses_resumes_and_kills_a_running_encode() {
        let folder = tempfile::tempdir().unwrap();
        let file = make_source(folder.path(), 20, "1280x720");
        let source = source_of(&file, 20.0, 1280, 720);
        let whole = encode_args::normalise(&[], 20.0);
        let output = folder.path().join("out.mp4");
        let args = encode_args::args(&source, &whole, 46, &output);
        let written = Arc::new(AtomicU64::new(0));
        let written_by_reader = written.clone();
        let encoder = Encoder::new().unwrap();
        let running = encoder
            .start(&args, move |seconds| {
                written_by_reader.store((seconds * 1000.0) as u64, Ordering::SeqCst);
            })
            .unwrap();
        let until = |done: &dyn Fn() -> bool| {
            let started = Instant::now();
            while !done() && started.elapsed() < Duration::from_secs(60) {
                thread::sleep(Duration::from_millis(50));
            }
            assert!(done(), "timed out");
        };

        until(&|| written.load(Ordering::SeqCst) > 0);
        running.pause().unwrap();
        thread::sleep(Duration::from_millis(500));
        let paused_at = written.load(Ordering::SeqCst);
        thread::sleep(Duration::from_millis(1500));
        assert_eq!(written.load(Ordering::SeqCst), paused_at);

        running.resume().unwrap();
        until(&|| written.load(Ordering::SeqCst) > paused_at);

        running.kill().unwrap();
        assert!(running.wait().is_err());
    }
}
