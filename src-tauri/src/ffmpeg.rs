use serde::Deserialize;
use std::io;
use std::path::Path;
use std::process::{self, Command};

#[derive(Debug, PartialEq)]
pub struct Probe {
    pub duration_s: f64,
    pub width: i64,
    pub height: i64,
    pub fps: f64,
}

#[derive(Deserialize)]
struct Output {
    streams: Vec<Stream>,
    format: Format,
}

#[derive(Deserialize)]
struct Stream {
    width: i64,
    height: i64,
    avg_frame_rate: String,
    r_frame_rate: String,
}

#[derive(Deserialize)]
struct Format {
    duration: String,
}

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

// A video's length, size and frame rate, or None when ffprobe can't read the file as a video.
pub fn probe(file: &Path) -> io::Result<Option<Probe>> {
    let mut ffprobe = Command::new("ffprobe");
    ffprobe
        .args(["-v", "error", "-select_streams", "v:0"])
        .args([
            "-show_entries",
            "stream=width,height,avg_frame_rate,r_frame_rate:format=duration",
            "-of",
            "json",
        ])
        .arg(file);

    let output = run(ffprobe)?;

    Ok(parse(&output.stdout))
}

fn run(mut tool: Command) -> io::Result<process::Output> {
    // A windowed app's console child opens a console window of its own unless told not to.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        tool.creation_flags(CREATE_NO_WINDOW);
    }

    match tool.output() {
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            Err(io::Error::new(e.kind(), "FFmpeg isn't installed"))
        }
        result => result,
    }
}

// The output decides, not the exit code: a file ffprobe fails on prints no video stream.
fn parse(json: &[u8]) -> Option<Probe> {
    let output: Output = serde_json::from_slice(json).ok()?;
    let video = output.streams.first()?;
    let duration_s = output.format.duration.parse().ok()?;

    // A variable-rate capture reports 0/0 as its average; its base rate then stands in.
    let fps = fraction(&video.avg_frame_rate).or_else(|| fraction(&video.r_frame_rate))?;

    Some(Probe {
        duration_s,
        width: video.width,
        height: video.height,
        fps,
    })
}

// "60000/1001" as 59.94; None for "0/0".
fn fraction(text: &str) -> Option<f64> {
    let (top, bottom) = text.split_once('/')?;
    let (top, bottom): (f64, f64) = (top.parse().ok()?, bottom.parse().ok()?);

    (top > 0.0 && bottom > 0.0).then(|| top / bottom)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_length_and_height_ffprobe_reports() {
        let json = br#"{
            "programs": [],
            "stream_groups": [],
            "streams": [{
                "width": 2560, "height": 1440,
                "avg_frame_rate": "60000/1001", "r_frame_rate": "60/1"
            }],
            "format": { "duration": "150.267133" }
        }"#;
        let probe = Probe {
            duration_s: 150.267133,
            width: 2560,
            height: 1440,
            fps: 60000.0 / 1001.0,
        };
        assert_eq!(parse(json), Some(probe));
    }

    #[test]
    fn a_zero_average_rate_falls_back_to_the_base_rate() {
        let json = br#"{
            "streams": [{
                "width": 1920, "height": 1080,
                "avg_frame_rate": "0/0", "r_frame_rate": "30/1"
            }],
            "format": { "duration": "3.0" }
        }"#;
        assert_eq!(parse(json).unwrap().fps, 30.0);
    }

    #[test]
    fn a_file_without_a_video_is_unreadable() {
        let no_video = br#"{ "streams": [], "format": { "duration": "3.0" } }"#;
        assert_eq!(parse(no_video), None);
        assert_eq!(parse(b""), None);
    }
}
