use crate::ffmpeg::{self, Probe};
use std::ffi::OsString;
use std::path::Path;

const DURATION_TOLERANCE: f64 = 0.02;

// The output has a video stream of the expected size and the length the clip keeps.
pub fn probes(file: &Path, size: (i64, i64), kept_s: f64) -> Result<(), String> {
    let probe = ffmpeg::probe(file).map_err(|e| e.to_string())?;

    shape(probe, size, kept_s)
}

// The arguments of the full decode: ffprobe reads only the container and the first frames, so a
// damaged payload passes `probes`. Run through the `Encoder`; any stderr text fails the check, as the
// exit code alone isn't trusted.
pub fn decode_args(file: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = ["-v", "error", "-xerror", "-i"].map(Into::into).into();
    args.push(file.into());
    args.extend(["-f", "null", "-", "-progress", "pipe:1", "-nostats"].map(OsString::from));

    args
}

fn shape(probe: Option<Probe>, size: (i64, i64), kept_s: f64) -> Result<(), String> {
    let Some(probe) = probe else {
        return Err("The output has no video stream".into());
    };

    if (probe.width, probe.height) != size {
        let (width, height) = (probe.width, probe.height);
        return Err(format!(
            "The output is {width}x{height}, not {}x{}",
            size.0, size.1
        ));
    }
    if (probe.duration_s - kept_s).abs() > kept_s * DURATION_TOLERANCE {
        return Err(format!(
            "The output runs {:.2} s, not {kept_s:.2} s",
            probe.duration_s
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn probe(width: i64, height: i64, duration_s: f64) -> Option<Probe> {
        Some(Probe {
            duration_s,
            width,
            height,
            fps: 60.0,
        })
    }

    #[test]
    fn a_right_sized_output_within_two_percent_passes() {
        assert_eq!(shape(probe(1280, 720, 10.15), (1280, 720), 10.0), Ok(()));
        assert_eq!(shape(probe(1280, 720, 9.85), (1280, 720), 10.0), Ok(()));
    }

    #[test]
    fn no_video_the_wrong_size_or_the_wrong_length_fails() {
        let missing = shape(None, (1280, 720), 10.0).unwrap_err();
        let small = shape(probe(640, 360, 10.0), (1280, 720), 10.0).unwrap_err();
        let short = shape(probe(1280, 720, 9.7), (1280, 720), 10.0).unwrap_err();

        assert_eq!(missing, "The output has no video stream");
        assert_eq!(small, "The output is 640x360, not 1280x720");
        assert_eq!(short, "The output runs 9.70 s, not 10.00 s");
    }
}
