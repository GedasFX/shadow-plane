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
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

const NOT_INSTALLED: &str = "FFmpeg isn't installed or isn't on PATH";

#[derive(Debug, Default, PartialEq)]
pub struct Colour {
    pub space: Option<String>,
    pub primaries: Option<String>,
    pub trc: Option<String>,
}

// What an encode's arguments need and the index lacks.
#[allow(dead_code)]
#[derive(Debug, PartialEq)]
pub struct Streams {
    pub audio_tracks: usize,
    pub colour: Colour,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Tagged {
    codec_type: Option<String>,
    color_space: Option<String>,
    color_primaries: Option<String>,
    color_transfer: Option<String>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct Tags {
    streams: Vec<Tagged>,
}

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

#[allow(dead_code)]
// The audio track count and the first video stream's colour tags.
pub fn streams(file: &Path) -> io::Result<Streams> {
    let mut ffprobe = Command::new("ffprobe");
    ffprobe
        .args(["-v", "error", "-show_entries"])
        .args([
            "stream=codec_type,color_space,color_primaries,color_transfer",
            "-of",
            "json",
        ])
        .arg(file);

    let output = run(ffprobe)?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(error.into_owned()));
    }

    parse_streams(&output.stdout).ok_or_else(|| io::Error::other("ffprobe printed no streams"))
}

// The video's first frame, 320 px wide, as a JPEG.
pub fn frame(video: &Path, jpeg: &Path) -> io::Result<()> {
    let mut ffmpeg = Command::new("ffmpeg");
    ffmpeg
        .args(["-v", "error", "-y", "-i"])
        .arg(video)
        .args(["-frames:v", "1", "-vf", "scale=320:-2", "-q:v", "5"])
        .arg(jpeg);

    let output = run(ffmpeg)?;
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(error.into_owned()));
    }

    Ok(())
}

pub fn run(mut tool: Command) -> io::Result<process::Output> {
    // A windowed app's console child opens a console window of its own unless told not to.
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        tool.creation_flags(CREATE_NO_WINDOW);
    }

    tool.output().map_err(not_installed)
}

pub fn not_installed(e: io::Error) -> io::Error {
    match e.kind() {
        io::ErrorKind::NotFound => io::Error::new(e.kind(), NOT_INSTALLED),
        _ => e,
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

#[allow(dead_code)]
fn parse_streams(json: &[u8]) -> Option<Streams> {
    let tags: Tags = serde_json::from_slice(json).ok()?;
    let is = |kind: &'static str| move |s: &&Tagged| s.codec_type.as_deref() == Some(kind);
    let known = |tag: &Option<String>| tag.clone().filter(|t| t != "unknown");

    let video = tags.streams.iter().find(is("video"))?;

    Some(Streams {
        audio_tracks: tags.streams.iter().filter(is("audio")).count(),
        colour: Colour {
            space: known(&video.color_space),
            primaries: known(&video.color_primaries),
            trc: known(&video.color_transfer),
        },
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
    fn counts_audio_tracks_and_keeps_known_colour_tags() {
        let json = br#"{ "streams": [
            { "codec_type": "video", "color_space": "smpte170m",
              "color_primaries": "unknown", "color_transfer": "bt709" },
            { "codec_type": "audio" }, { "codec_type": "audio" }
        ] }"#;
        let streams = parse_streams(json).unwrap();

        assert_eq!(streams.audio_tracks, 2);
        assert_eq!(streams.colour.space.as_deref(), Some("smpte170m"));
        assert_eq!(streams.colour.primaries, None);
        assert_eq!(streams.colour.trc.as_deref(), Some("bt709"));
    }

    #[test]
    fn a_missing_ffmpeg_says_so_plainly() {
        let missing = not_installed(io::Error::from(io::ErrorKind::NotFound));
        assert_eq!(
            missing.to_string(),
            "FFmpeg isn't installed or isn't on PATH"
        );
    }

    #[test]
    fn a_file_without_a_video_is_unreadable() {
        let no_video = br#"{ "streams": [], "format": { "duration": "3.0" } }"#;
        assert_eq!(parse(no_video), None);
        assert_eq!(parse(b""), None);
    }
}
