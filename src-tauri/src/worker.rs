use crate::bin;
use crate::clip_name;
use crate::encode_args::{self, Source};
use crate::ffmpeg::Streams;
use crate::files;
use crate::jobs;
use crate::media;
use crate::settings;
use crate::verify;
use rusqlite::Connection;
use serde::Serialize;
use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub const PROCESSED: &str = "_Processed";

const CRF: u32 = 46;
const PROGRESS_EVERY: Duration = Duration::from_millis(250);
const GONE: &str = "The video isn't in the library any more";

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Encode,
    Check,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Progress {
    pub id: i64,
    pub phase: Phase,
    pub done_s: f64,
    pub kept_s: f64,
    // From the phase's own rate, paused time left out; None until there is a rate.
    pub left_s: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    QueueChanged,
    Progress(Progress),
}

pub trait Control: Send + Sync {
    fn pause(&self) -> io::Result<()>;
    fn resume(&self) -> io::Result<()>;
    fn kill(&self) -> io::Result<()>;
}

pub struct Process {
    pub control: Arc<dyn Control>,
    // Blocks until the process ends; Ok holds its stderr tail.
    pub wait: Box<dyn FnOnce() -> Result<String, String> + Send>,
}

// What the worker needs of ffmpeg, so a test can stand in for it.
pub trait Tools: Send + Sync + 'static {
    fn streams(&self, file: &Path) -> io::Result<Streams>;
    // `on_progress` gets the seconds of output written so far, on a thread of its own.
    fn start(
        &self,
        args: &[OsString],
        on_progress: Box<dyn FnMut(f64) + Send>,
    ) -> io::Result<Process>;
    fn probes(&self, file: &Path, size: (i64, i64), kept_s: f64) -> Result<(), String>;
}

#[cfg(windows)]
pub struct Real(pub crate::encoder::Encoder);

#[cfg(windows)]
impl Control for crate::encoder::Control {
    fn pause(&self) -> io::Result<()> {
        self.pause()
    }

    fn resume(&self) -> io::Result<()> {
        self.resume()
    }

    fn kill(&self) -> io::Result<()> {
        self.kill()
    }
}

#[cfg(windows)]
impl Tools for Real {
    fn streams(&self, file: &Path) -> io::Result<Streams> {
        crate::ffmpeg::streams(file)
    }

    fn start(
        &self,
        args: &[OsString],
        on_progress: Box<dyn FnMut(f64) + Send>,
    ) -> io::Result<Process> {
        let running = self.0.start(args, on_progress)?;
        let control = Arc::new(running.control());

        Ok(Process {
            control,
            wait: Box::new(move || running.wait()),
        })
    }

    fn probes(&self, file: &Path, size: (i64, i64), kept_s: f64) -> Result<(), String> {
        verify::probes(file, size, kept_s)
    }
}

// Rate and throttle of one phase's progress; the clock is passed in.
struct Tracker {
    id: i64,
    phase: Phase,
    kept_s: f64,
    started: Instant,
    paused_total: Duration,
    paused_since: Option<Instant>,
    last_sent: Option<Instant>,
}

impl Tracker {
    fn new(id: i64, phase: Phase, kept_s: f64, now: Instant) -> Tracker {
        Tracker {
            id,
            phase,
            kept_s,
            started: now,
            paused_total: Duration::ZERO,
            paused_since: None,
            last_sent: None,
        }
    }

    fn pause(&mut self, now: Instant) {
        self.paused_since.get_or_insert(now);
    }

    fn resume(&mut self, now: Instant) {
        if let Some(since) = self.paused_since.take() {
            self.paused_total += now - since;
        }
    }

    // None when one went out less than a quarter second ago.
    fn sample(&mut self, now: Instant, done_s: f64) -> Option<Progress> {
        if self.last_sent.is_some_and(|at| now - at < PROGRESS_EVERY) {
            return None;
        }
        self.last_sent = Some(now);

        let done_s = done_s.min(self.kept_s);
        let paused = self.paused_total + self.paused_since.map_or(Duration::ZERO, |s| now - s);
        let active_s = (now - self.started).saturating_sub(paused).as_secs_f64();
        let left_s =
            (done_s > 0.0 && active_s > 0.0).then(|| (self.kept_s - done_s) / (done_s / active_s));

        Some(Progress {
            id: self.id,
            phase: self.phase,
            done_s,
            kept_s: self.kept_s,
            left_s,
        })
    }
}

#[derive(Default)]
struct State {
    paused: bool,
    active: Option<Active>,
}

// The job being worked on.
struct Active {
    id: i64,
    cancelled: bool,
    frozen: bool,
    // Set while an ffmpeg of the job runs.
    running: Option<Live>,
}

struct Live {
    control: Arc<dyn Control>,
    tracker: Arc<Mutex<Tracker>>,
}

struct Shared {
    db: Arc<Mutex<Connection>>,
    tools: Box<dyn Tools>,
    stage: PathBuf,
    emit: Box<dyn Fn(Event) + Send + Sync>,
    state: Mutex<State>,
    wake: Condvar,
}

enum Stop {
    Cancelled,
    Failed(String),
}

fn failed(e: impl ToString) -> Stop {
    Stop::Failed(e.to_string())
}

// Encodes the saved clips one at a time, in queue order, into the library's _Processed folder.
#[derive(Clone)]
pub struct Worker {
    shared: Arc<Shared>,
}

impl Worker {
    // A job left running by the last exit goes back in line, and the stage folder, which holds only
    // that exit's leftovers, is emptied.
    pub fn new(
        db: Arc<Mutex<Connection>>,
        tools: impl Tools,
        stage: PathBuf,
        emit: impl Fn(Event) + Send + Sync + 'static,
    ) -> rusqlite::Result<Worker> {
        jobs::reset_running(&db.lock().unwrap())?;
        let _ = fs::remove_dir_all(&stage);

        Ok(Worker {
            shared: Arc::new(Shared {
                db,
                tools: Box::new(tools),
                stage,
                emit: Box::new(emit),
                state: Mutex::default(),
                wake: Condvar::new(),
            }),
        })
    }

    pub fn spawn(&self) {
        let worker = self.clone();
        thread::spawn(move || {
            loop {
                let mut state = worker.shared.state.lock().unwrap();
                let (id, root) = loop {
                    if let Some(next) = worker.claim(&mut state) {
                        break next;
                    }
                    state = worker.shared.wake.wait(state).unwrap();
                };
                drop(state);
                worker.work(id, &root);
            }
        });
    }

    pub fn paused(&self) -> bool {
        self.shared.state.lock().unwrap().paused
    }

    // The queue changed: tells the page and wakes the worker.
    pub fn changed(&self) {
        self.announce();
        self.wake();
    }

    fn announce(&self) {
        (self.shared.emit)(Event::QueueChanged);
    }

    pub fn wake(&self) {
        // Under the lock the worker holds while it looks for work, so the wake can't fall between.
        let _state = self.shared.state.lock().unwrap();
        self.shared.wake.notify_all();
    }

    pub fn pause(&self) -> Result<(), String> {
        let frozen = {
            let mut state = self.shared.state.lock().unwrap();
            state.paused = true;
            match state.active.as_mut() {
                Some(active) if !active.frozen => active.freeze(Instant::now()),
                _ => Ok(()),
            }
        };
        self.changed();

        frozen.map_err(|e| e.to_string())
    }

    pub fn resume(&self) -> Result<(), String> {
        let thawed = {
            let mut state = self.shared.state.lock().unwrap();
            state.paused = false;
            match state.active.as_mut() {
                Some(active) if active.frozen => active.thaw(Instant::now()),
                _ => Ok(()),
            }
        };
        self.changed();

        thawed.map_err(|e| e.to_string())
    }

    // The running job is killed and ends cancelled; a pending one is removed.
    pub fn cancel(&self, id: i64) -> Result<(), String> {
        {
            let mut state = self.shared.state.lock().unwrap();
            if let Some(active) = state.active.as_mut().filter(|a| a.id == id) {
                active.cancelled = true;
                if let Some(Live { control, .. }) = &active.running {
                    control.kill().map_err(|e| e.to_string())?;
                }
                return Ok(());
            }
        }

        jobs::remove(&self.shared.db.lock().unwrap(), id).map_err(|e| e.to_string())?;
        self.changed();

        Ok(())
    }

    // Takes the first pending job if the queue isn't paused and a library folder is chosen. For a test.
    #[cfg(test)]
    fn run_next(&self) -> bool {
        let next = self.claim(&mut self.shared.state.lock().unwrap());

        match next {
            Some((id, root)) => {
                self.work(id, &root);
                true
            }
            None => false,
        }
    }

    // The claim and the active job are set under one state lock, so a pause or a cancel finds it.
    fn claim(&self, state: &mut State) -> Option<(i64, PathBuf)> {
        if state.paused {
            return None;
        }

        let db = self.shared.db.lock().unwrap();
        let root = settings::get(&db, settings::LIBRARY).ok().flatten()?;
        let id = jobs::claim(&db).ok().flatten()?;
        state.active = Some(Active {
            id,
            cancelled: false,
            frozen: false,
            running: None,
        });

        Some((id, PathBuf::from(root)))
    }

    fn work(&self, id: i64, root: &Path) {
        self.announce();
        let staged = self.shared.stage.join(format!("{id}.mp4"));
        let outcome = self.encode(id, root, &staged);
        let _ = fs::remove_file(&staged);

        let now = chrono::Utc::now().to_rfc3339();
        let (status, output, error) = match &outcome {
            Ok(output) => ("done", Some(output.as_str()), None),
            Err(Stop::Cancelled) => ("cancelled", None, None),
            Err(Stop::Failed(error)) => ("failed", None, Some(error.as_str())),
        };
        let finished = jobs::finish(
            &self.shared.db.lock().unwrap(),
            id,
            status,
            output,
            error,
            &now,
        );
        self.shared.state.lock().unwrap().active = None;
        self.announce();

        if finished.is_ok() && outcome.is_ok() {
            self.bin_if_all_done(id, root);
        }
        self.wake();
    }

    // The original goes to _Bin once every clip of its video is made.
    fn bin_if_all_done(&self, id: i64, root: &Path) {
        let db = &self.shared.db;
        let key = jobs::get(&db.lock().unwrap(), id)
            .ok()
            .flatten()
            .map(|j| j.key);
        let Some(key) = key else { return };
        if !jobs::all_done(&db.lock().unwrap(), &key).unwrap_or(false) {
            return;
        }

        if let Err(e) = bin::bin(db, root, &key) {
            let error = format!("The original stayed in place: {e}");
            let _ = jobs::set_error(&db.lock().unwrap(), id, &error);
        }
        self.announce();
    }

    fn encode(&self, id: i64, root: &Path, staged: &Path) -> Result<String, Stop> {
        let (job, source, pieces) = {
            let db = self.shared.db.lock().unwrap();
            let job = jobs::get(&db, id)
                .map_err(failed)?
                .ok_or_else(|| failed(GONE))?;
            let source = jobs::source(&db, &job.key).map_err(failed)?;
            (job, source, jobs::pieces(&db, id).map_err(failed)?)
        };
        let source = source.ok_or_else(|| failed(GONE))?;
        let file = media::below(root, &job.key)
            .map(|file| media::in_bin_if_gone(root, &job.key, file))
            .filter(|file| file.exists())
            .ok_or_else(|| failed(GONE))?;

        let streams = self.shared.tools.streams(&file).map_err(failed)?;
        let pieces = encode_args::normalise(&pieces, source.duration_s);
        let kept_s: f64 = pieces.iter().map(|p| p.end_s - p.start_s).sum();
        fs::create_dir_all(&self.shared.stage).map_err(failed)?;
        let input = Source {
            path: &file,
            duration_s: source.duration_s,
            width: source.width,
            height: source.height,
            fps: source.fps,
            audio_tracks: streams.audio_tracks,
            colour: streams.colour,
        };
        let args = encode_args::args(&input, &pieces, CRF, staged);

        self.run(id, Phase::Encode, &args, kept_s)?;
        self.not_cancelled()?;
        let size = encode_args::output_size(source.width, source.height);
        self.shared
            .tools
            .probes(staged, size, kept_s)
            .map_err(failed)?;
        self.not_cancelled()?;

        let errors = self.run(id, Phase::Check, &verify::decode_args(staged), kept_s)?;
        if !errors.trim().is_empty() {
            return Err(failed(errors));
        }
        self.not_cancelled()?;

        publish(root, staged, &job.game, &job.file_name).map_err(failed)
    }

    fn not_cancelled(&self) -> Result<(), Stop> {
        let state = self.shared.state.lock().unwrap();

        match state.active.as_ref().is_some_and(|a| a.cancelled) {
            true => Err(Stop::Cancelled),
            false => Ok(()),
        }
    }

    // One ffmpeg of the job, registered so pause, resume and cancel reach it; Ok is its stderr tail.
    fn run(&self, id: i64, phase: Phase, args: &[OsString], kept_s: f64) -> Result<String, Stop> {
        let tracker = Arc::new(Mutex::new(Tracker::new(id, phase, kept_s, Instant::now())));
        let on_progress = {
            let tracker = tracker.clone();
            let shared = self.shared.clone();
            move |done_s: f64| {
                let sample = tracker.lock().unwrap().sample(Instant::now(), done_s);
                if let Some(progress) = sample {
                    (shared.emit)(Event::Progress(progress));
                }
            }
        };
        let process = self
            .shared
            .tools
            .start(args, Box::new(on_progress))
            .map_err(failed)?;

        {
            let mut state = self.shared.state.lock().unwrap();
            let paused = state.paused;
            let active = state.active.as_mut().expect("a job being worked on");
            active.running = Some(Live {
                control: process.control.clone(),
                tracker,
            });
            if active.cancelled {
                let _ = process.control.kill();
            } else if paused {
                // Pause told the page already if this fails.
                let _ = active.freeze(Instant::now());
            }
        }

        let result = (process.wait)();
        {
            let mut state = self.shared.state.lock().unwrap();
            let active = state.active.as_mut().expect("a job being worked on");
            active.running = None;
            active.frozen = false;
            if active.cancelled {
                return Err(Stop::Cancelled);
            }
        }

        result.map_err(Stop::Failed)
    }
}

impl Active {
    fn freeze(&mut self, now: Instant) -> io::Result<()> {
        let Some(Live { control, tracker }) = &self.running else {
            return Ok(());
        };
        control.pause()?;
        tracker.lock().unwrap().pause(now);
        self.frozen = true;

        Ok(())
    }

    fn thaw(&mut self, now: Instant) -> io::Result<()> {
        let Some(Live { control, tracker }) = &self.running else {
            return Ok(());
        };
        control.resume()?;
        tracker.lock().unwrap().resume(now);
        self.frozen = false;

        Ok(())
    }
}

// Copies the staged file to _Processed/<game>/<file name>, ` (2)`, ` (3)` … when taken; the output's
// key. An existing file is never overwritten, and a copy that differs is deleted.
fn publish(root: &Path, staged: &Path, game: &str, file_name: &str) -> io::Result<String> {
    let folder = match clip_name::sanitize(game).as_str() {
        "" => "Misc".to_string(),
        game => game.to_string(),
    };
    let dir = root.join(PROCESSED).join(&folder);
    fs::create_dir_all(&dir)?;

    let name = Path::new(file_name);
    let stem = name
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    let ext = name
        .extension()
        .map_or_else(String::new, |e| format!(".{}", e.to_string_lossy()));
    for n in 1.. {
        let candidate = match n {
            1 => file_name.to_string(),
            n => format!("{stem} ({n}){ext}"),
        };
        match files::copy_verified(staged, &dir.join(&candidate)) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            done => return done.map(|()| format!("{PROCESSED}/{folder}/{candidate}")),
        }
    }

    unreachable!("the loop returns")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::ffmpeg::Colour;
    use crate::review::{self, ClipInput, Piece};
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    const PEAK: &str = "PEAK/PEAK 2024.02.20 - 23.46.01.02.DVR.mp4";
    const OTHER: &str = "PEAK/PEAK 2024.02.21 - 20.00.00.02.DVR.mp4";

    #[derive(Default)]
    struct Fake {
        // Hold every encode until it is killed or released.
        hold: AtomicBool,
        release: AtomicBool,
        fail_encode: AtomicBool,
        bad_decode: AtomicBool,
        live: AtomicUsize,
        most_live: AtomicUsize,
        paused: AtomicBool,
        // The job ids encoded, in order.
        started: Mutex<Vec<String>>,
    }

    struct FakeControl(Arc<Fake>, AtomicBool);

    impl Control for FakeControl {
        fn pause(&self) -> io::Result<()> {
            self.0.paused.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn resume(&self) -> io::Result<()> {
            self.0.paused.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn kill(&self) -> io::Result<()> {
            self.1.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    impl Tools for Arc<Fake> {
        fn streams(&self, _: &Path) -> io::Result<Streams> {
            Ok(Streams {
                audio_tracks: 1,
                colour: Colour::default(),
            })
        }

        fn start(
            &self,
            args: &[OsString],
            mut on_progress: Box<dyn FnMut(f64) + Send>,
        ) -> io::Result<Process> {
            let checking = args.iter().any(|a| a == "null");
            let fake = self.clone();
            if !checking {
                let output = Path::new(args.last().unwrap());
                fs::write(output, b"encoded")?;
                let id = output.file_stem().unwrap().to_string_lossy().into_owned();
                fake.started.lock().unwrap().push(id);
            }
            let live = fake.live.fetch_add(1, Ordering::SeqCst) + 1;
            fake.most_live.fetch_max(live, Ordering::SeqCst);
            on_progress(1.0);

            let killed = Arc::new(FakeControl(fake.clone(), AtomicBool::new(false)));
            let control = killed.clone();
            let wait = move || {
                while fake.hold.load(Ordering::SeqCst)
                    && !fake.release.load(Ordering::SeqCst)
                    && !killed.1.load(Ordering::SeqCst)
                {
                    thread::sleep(Duration::from_millis(5));
                }
                fake.live.fetch_sub(1, Ordering::SeqCst);

                if killed.1.load(Ordering::SeqCst) {
                    Err("killed".to_string())
                } else if !checking && fake.fail_encode.load(Ordering::SeqCst) {
                    Err("ffmpeg failed: bad input".to_string())
                } else if checking && fake.bad_decode.load(Ordering::SeqCst) {
                    Ok("invalid data found".to_string())
                } else {
                    Ok(String::new())
                }
            };

            Ok(Process {
                control,
                wait: Box::new(wait),
            })
        }

        fn probes(&self, _: &Path, _: (i64, i64), _: f64) -> Result<(), String> {
            Ok(())
        }
    }

    struct Setup {
        root: tempfile::TempDir,
        stage: tempfile::TempDir,
        db: Arc<Mutex<Connection>>,
        fake: Arc<Fake>,
        worker: Worker,
        events: Arc<Mutex<Vec<Event>>>,
    }

    // A library of two 60 s videos on disk and in the index.
    fn setup() -> Setup {
        let root = tempfile::tempdir().unwrap();
        let stage = tempfile::tempdir().unwrap();
        let conn = db::memory();
        settings::set(&conn, settings::LIBRARY, root.path().to_str().unwrap()).unwrap();
        for key in [PEAK, OTHER] {
            let file = root.path().join(key);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(&file, key).unwrap();
            conn.execute(
                "INSERT INTO source (path, size, modified_ms, duration_s, width, height, fps) VALUES (?1, 500, 1, 60, 2560, 1440, 59.94)",
                [key],
            )
            .unwrap();
        }
        let db = Arc::new(Mutex::new(conn));
        let fake = Arc::new(Fake::default());
        let events = Arc::new(Mutex::new(Vec::new()));
        let log = events.clone();
        let worker = Worker::new(
            db.clone(),
            fake.clone(),
            stage.path().join("stage"),
            move |event| log.lock().unwrap().push(event),
        )
        .unwrap();

        Setup {
            root,
            stage,
            db,
            fake,
            worker,
            events,
        }
    }

    impl Setup {
        fn save(&self, key: &str, title: &str) -> i64 {
            let clip = ClipInput {
                title: title.into(),
                game: "PEAK".into(),
                great: false,
                best: false,
                pieces: vec![Piece {
                    start_s: 1.0,
                    end_s: 5.0,
                }],
            };
            let mut conn = self.db.lock().unwrap();
            review::save(&mut conn, key, &clip, "2026-11-06T10:00:00Z").unwrap();

            conn.query_row("SELECT max(id) FROM job", [], |row| row.get(0))
                .unwrap()
        }

        fn job(&self, id: i64) -> jobs::Job {
            jobs::get(&self.db.lock().unwrap(), id).unwrap().unwrap()
        }

        fn status(&self, id: i64) -> String {
            self.job(id).status
        }

        fn processed(&self) -> Vec<String> {
            let dir = self.root.path().join(PROCESSED).join("PEAK");
            let mut names: Vec<String> = fs::read_dir(dir)
                .map(|d| {
                    d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                        .collect()
                })
                .unwrap_or_default();
            names.sort();
            names
        }

        fn staged(&self) -> Vec<PathBuf> {
            fs::read_dir(self.stage.path().join("stage"))
                .map(|d| d.map(|e| e.unwrap().path()).collect())
                .unwrap_or_default()
        }

        fn in_bin(&self, key: &str) -> bool {
            self.root.path().join("_Bin").join(key).exists()
        }
    }

    fn until(mut done: impl FnMut() -> bool) {
        let started = Instant::now();
        while !done() && started.elapsed() < Duration::from_secs(10) {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(done(), "timed out");
    }

    #[test]
    fn jobs_run_in_queue_order_one_at_a_time() {
        let s = setup();
        let a = s.save(PEAK, "a");
        let b = s.save(PEAK, "b");
        let c = s.save(OTHER, "c");
        jobs::reorder(&mut s.db.lock().unwrap(), &[c, a, b]).unwrap();
        s.fake.hold.store(true, Ordering::SeqCst);
        s.fake.release.store(true, Ordering::SeqCst);

        s.worker.spawn();
        s.worker.wake();
        until(|| [a, b, c].iter().all(|&id| s.status(id) == "done"));

        let order: Vec<String> = s.fake.started.lock().unwrap().clone();
        assert_eq!(order, [c, a, b].map(|id| id.to_string()));
        assert_eq!(s.fake.most_live.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_finished_clip_lands_in_processed_and_a_taken_name_gets_a_number() {
        let s = setup();
        let one = s.save(PEAK, "same");
        let two = s.save(PEAK, "same");

        assert!(s.worker.run_next() && s.worker.run_next() && !s.worker.run_next());

        let (one, two) = (s.job(one), s.job(two));
        let name = one.file_name.clone();
        let numbered = name.replace(".mp4", " (2).mp4");
        assert_eq!(one.output.unwrap(), format!("_Processed/PEAK/{name}"));
        assert_eq!(two.output.unwrap(), format!("_Processed/PEAK/{numbered}"));
        assert_eq!(s.processed(), {
            let mut both = vec![name, numbered];
            both.sort();
            both
        });
        assert_eq!(
            fs::read(
                s.root
                    .path()
                    .join(PROCESSED)
                    .join("PEAK")
                    .join(&one.file_name)
            )
            .unwrap(),
            b"encoded"
        );
        assert!(s.staged().is_empty());
        assert_eq!((one.error, two.status.as_str()), (None, "done"));
    }

    #[test]
    fn a_failed_encode_keeps_the_original_deletes_the_staged_file_and_the_next_job_runs() {
        let s = setup();
        let bad = s.save(PEAK, "bad");
        let good = s.save(OTHER, "good");
        s.fake.fail_encode.store(true, Ordering::SeqCst);

        s.worker.run_next();
        s.fake.fail_encode.store(false, Ordering::SeqCst);
        s.worker.run_next();

        let bad = s.job(bad);
        assert_eq!(bad.status, "failed");
        assert!(bad.error.unwrap().contains("bad input"));
        assert_eq!(s.status(good), "done");
        assert!(s.root.path().join(PEAK).exists() && !s.in_bin(PEAK));
        assert_eq!(s.processed().len(), 1);
        assert!(s.staged().is_empty());
    }

    #[test]
    fn a_decode_error_fails_the_job_and_publishes_nothing() {
        let s = setup();
        let id = s.save(PEAK, "a");
        s.fake.bad_decode.store(true, Ordering::SeqCst);

        s.worker.run_next();

        assert_eq!(s.status(id), "failed");
        assert_eq!(s.job(id).error.as_deref(), Some("invalid data found"));
        assert!(s.processed().is_empty() && s.staged().is_empty());
        assert!(s.root.path().join(PEAK).exists());
    }

    #[test]
    fn a_video_gone_from_the_library_fails_its_job() {
        let s = setup();
        let id = s.save(PEAK, "a");
        fs::remove_file(s.root.path().join(PEAK)).unwrap();

        s.worker.run_next();

        assert_eq!(s.status(id), "failed");
        assert_eq!(
            s.job(id).error.as_deref(),
            Some("The video isn't in the library any more")
        );
    }

    #[test]
    fn a_binned_video_is_encoded_from_bin() {
        let s = setup();
        let id = s.save(PEAK, "a");
        let bin = s.root.path().join("_Bin").join(PEAK);
        fs::create_dir_all(bin.parent().unwrap()).unwrap();
        fs::rename(s.root.path().join(PEAK), bin).unwrap();

        s.worker.run_next();

        assert_eq!(s.status(id), "done");
    }

    #[test]
    fn the_original_goes_to_bin_when_every_clip_of_the_video_is_done() {
        let s = setup();
        let a = s.save(PEAK, "a");
        let b = s.save(PEAK, "b");
        s.save(OTHER, "c");

        s.worker.run_next();
        assert!(!s.in_bin(PEAK), "one clip of the video is still pending");
        s.worker.run_next();
        assert!(s.in_bin(PEAK) && !s.root.path().join(PEAK).exists());
        assert_eq!(
            (s.status(a).as_str(), s.status(b).as_str()),
            ("done", "done")
        );
        assert_eq!(s.job(b).error, None);
    }

    #[test]
    fn one_failed_clip_keeps_the_original_in_place() {
        let s = setup();
        s.save(PEAK, "a");
        s.save(PEAK, "b");
        s.fake.fail_encode.store(true, Ordering::SeqCst);
        s.worker.run_next();
        s.fake.fail_encode.store(false, Ordering::SeqCst);

        s.worker.run_next();

        assert!(!s.in_bin(PEAK));
        assert!(s.root.path().join(PEAK).exists());
    }

    #[test]
    fn a_failed_move_to_bin_leaves_the_job_done_with_a_note() {
        let s = setup();
        let id = s.save(PEAK, "a");
        let taken = s.root.path().join("_Bin").join(PEAK);
        fs::create_dir_all(taken.parent().unwrap()).unwrap();
        fs::write(taken, "another").unwrap();

        s.worker.run_next();

        let job = s.job(id);
        assert_eq!(job.status, "done");
        assert!(
            job.error
                .unwrap()
                .starts_with("The original stayed in place")
        );
        assert!(s.root.path().join(PEAK).exists());
    }

    #[test]
    fn cancelling_the_running_job_kills_it_and_deletes_the_staged_file() {
        let s = setup();
        let id = s.save(PEAK, "a");
        let next = s.save(OTHER, "b");
        s.fake.hold.store(true, Ordering::SeqCst);
        s.worker.spawn();
        s.worker.wake();
        until(|| s.status(id) == "running" && s.fake.live.load(Ordering::SeqCst) == 1);

        s.worker.cancel(id).unwrap();
        s.fake.release.store(true, Ordering::SeqCst);

        until(|| s.status(next) == "done");
        assert_eq!(s.status(id), "cancelled");
        assert!(s.processed().len() == 1 && s.staged().is_empty());
        assert!(s.root.path().join(PEAK).exists());
    }

    #[test]
    fn cancelling_a_pending_job_removes_it() {
        let s = setup();
        let id = s.save(PEAK, "a");

        s.worker.cancel(id).unwrap();

        assert_eq!(s.status(id), "cancelled");
        assert!(!s.worker.run_next());
    }

    #[test]
    fn pause_holds_the_queue_and_freezes_the_running_encode_until_resume() {
        let s = setup();
        s.worker.pause().unwrap();
        let first = s.save(PEAK, "a");
        s.worker.spawn();
        s.worker.changed();
        thread::sleep(Duration::from_millis(200));
        assert_eq!(s.status(first), "pending");
        assert!(s.worker.paused());

        s.worker.resume().unwrap();
        s.fake.hold.store(true, Ordering::SeqCst);
        let second = s.save(OTHER, "b");
        s.worker.changed();
        until(|| s.status(first) == "running" && s.fake.live.load(Ordering::SeqCst) == 1);

        s.worker.pause().unwrap();
        assert!(s.fake.paused.load(Ordering::SeqCst));
        s.worker.resume().unwrap();
        assert!(!s.fake.paused.load(Ordering::SeqCst));
        s.worker.pause().unwrap();
        s.fake.release.store(true, Ordering::SeqCst);
        until(|| s.status(first) == "done");
        thread::sleep(Duration::from_millis(200));
        assert_eq!(s.status(second), "pending");

        s.worker.resume().unwrap();
        until(|| s.status(second) == "done");
    }

    #[test]
    fn the_queue_waits_for_a_library_folder() {
        let s = setup();
        s.save(PEAK, "a");
        s.db.lock()
            .unwrap()
            .execute("DELETE FROM setting", [])
            .unwrap();

        assert!(!s.worker.run_next());
    }

    #[test]
    fn at_launch_a_running_job_goes_back_in_line_and_the_stage_is_emptied() {
        let s = setup();
        let id = s.save(PEAK, "a");
        s.db.lock()
            .unwrap()
            .execute("UPDATE job SET status = 'running'", [])
            .unwrap();
        let stage = s.stage.path().join("stage");
        fs::create_dir_all(&stage).unwrap();
        fs::write(stage.join("9.mp4"), "leftover").unwrap();

        let worker = Worker::new(s.db.clone(), s.fake.clone(), stage, |_| {}).unwrap();

        assert_eq!(s.status(id), "pending");
        assert!(s.staged().is_empty());
        assert!(!worker.paused());
    }

    #[test]
    fn the_page_hears_of_each_change_and_of_progress() {
        let s = setup();
        s.save(PEAK, "a");

        s.worker.run_next();

        let events = s.events.lock().unwrap();
        assert!(events.contains(&Event::QueueChanged));
        let phases: Vec<Phase> = events
            .iter()
            .filter_map(|e| match e {
                Event::Progress(p) => Some(p.phase),
                _ => None,
            })
            .collect();
        assert_eq!(phases, [Phase::Encode, Phase::Check]);
    }

    #[test]
    fn progress_comes_at_most_four_a_second_with_a_rate_that_skips_paused_time() {
        let t0 = Instant::now();
        let at = |ms| t0 + Duration::from_millis(ms);
        let mut tracker = Tracker::new(1, Phase::Encode, 100.0, t0);

        assert_eq!(tracker.sample(at(0), 0.0).unwrap().left_s, None);
        assert!(tracker.sample(at(100), 1.0).is_none());
        // 10 s of output in 5 s of work: 10 s/s, 80 s to go.
        let first = tracker.sample(at(5000), 10.0).unwrap();
        assert_eq!((first.done_s, first.left_s), (10.0, Some(45.0)));
        tracker.pause(at(6000));
        tracker.resume(at(66000));
        // 20 s of output in 10 s of work, though 70 s have passed.
        let later = tracker.sample(at(70000), 20.0).unwrap();
        assert_eq!(later.left_s, Some(40.0));
        assert_eq!(tracker.sample(at(80000), 150.0).unwrap().done_s, 100.0);
    }

    #[cfg(windows)]
    #[test]
    fn a_real_clip_is_encoded_through_the_queue_into_processed() {
        use crate::{encoder::Encoder, ffmpeg};
        use std::process::Command;

        let root = tempfile::tempdir().unwrap();
        let file = root.path().join(PEAK);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        let mut ffmpeg = Command::new("ffmpeg");
        ffmpeg
            .args(["-v", "error", "-y", "-f", "lavfi", "-i"])
            .arg("testsrc2=size=320x180:rate=30:duration=4")
            .args(["-f", "lavfi", "-i", "sine=duration=4"])
            .args(["-c:v", "mpeg4", "-pix_fmt", "yuv420p", "-c:a", "aac"])
            .arg(&file);
        assert!(ffmpeg::run(ffmpeg).unwrap().status.success());
        let probe = ffmpeg::probe(&file).unwrap().unwrap();
        let conn = db::memory();
        settings::set(&conn, settings::LIBRARY, root.path().to_str().unwrap()).unwrap();
        conn.execute(
            "INSERT INTO source (path, size, modified_ms, duration_s, width, height, fps) VALUES (?1, 1, 1, ?2, ?3, ?4, ?5)",
            (PEAK, probe.duration_s, probe.width, probe.height, probe.fps),
        )
        .unwrap();
        let db = Arc::new(Mutex::new(conn));
        let events = Arc::new(Mutex::new(Vec::new()));
        let log = events.clone();
        let stage = root.path().join("stage");
        let worker = Worker::new(
            db.clone(),
            Real(Encoder::new().unwrap()),
            stage.clone(),
            move |event| log.lock().unwrap().push(event),
        )
        .unwrap();
        let clip = ClipInput {
            title: "real".into(),
            game: "PEAK".into(),
            great: false,
            best: false,
            pieces: vec![Piece {
                start_s: 0.5,
                end_s: 2.5,
            }],
        };
        review::save(&mut db.lock().unwrap(), PEAK, &clip, "2026-11-06T10:00:00Z").unwrap();

        assert!(worker.run_next());

        let job = jobs::get(&db.lock().unwrap(), 1).unwrap().unwrap();
        assert_eq!((job.status.as_str(), job.error), ("done", None));
        let output = root.path().join(job.output.unwrap());
        verify::probes(&output, (320, 180), 2.0).unwrap();
        assert!(root.path().join("_Bin").join(PEAK).exists());
        assert!(!stage.join("1.mp4").exists());
        assert!(
            events
                .lock()
                .unwrap()
                .iter()
                .any(|e| matches!(e, Event::Progress(_)))
        );
    }
}
