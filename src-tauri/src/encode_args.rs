use crate::ffmpeg::Colour;
use crate::review::Piece;
use std::ffi::OsString;
use std::path::Path;

const MAX_FPS: f64 = 60.0;
const MIN_PIECE_S: f64 = 0.05;
const MAIN_AUDIO_KBPS: u32 = 96;
const EXTRA_AUDIO_KBPS: u32 = 64;

pub struct Source<'a> {
    pub path: &'a Path,
    pub duration_s: f64,
    pub width: i64,
    pub height: i64,
    pub fps: f64,
    pub audio_tracks: usize,
    pub colour: Colour,
}

// Sorted, clamped to the video, overlaps trimmed, slivers dropped; the whole video when nothing is left.
pub fn normalise(pieces: &[Piece], duration_s: f64) -> Vec<Piece> {
    let mut sorted: Vec<(f64, f64)> = pieces
        .iter()
        .map(|p| {
            (
                p.start_s.clamp(0.0, duration_s),
                p.end_s.clamp(0.0, duration_s),
            )
        })
        .collect();
    sorted.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut kept: Vec<Piece> = Vec::new();
    for (mut start_s, end_s) in sorted {
        if let Some(last) = kept.last() {
            start_s = start_s.max(last.end_s);
        }
        if end_s - start_s > MIN_PIECE_S {
            kept.push(Piece {
                start_s: millis(start_s),
                end_s: millis(end_s),
            });
        }
    }

    if kept.is_empty() {
        kept.push(Piece {
            start_s: 0.0,
            end_s: duration_s,
        });
    }

    kept
}

// The picture size of the output: yuv420p needs even sides, so an odd one loses a pixel.
pub fn output_size(width: i64, height: i64) -> (i64, i64) {
    (width / 2 * 2, height / 2 * 2)
}

// The ffmpeg arguments for one clip; `pieces` come from `normalise`.
pub fn args(source: &Source, pieces: &[Piece], crf: u32, output: &Path) -> Vec<OsString> {
    let mut args: Vec<OsString> = Vec::new();
    let tracks = source.audio_tracks;

    let fps = source.fps.round().clamp(1.0, MAX_FPS);
    let mut tail = Vec::new();
    if source.width % 2 != 0 || source.height % 2 != 0 {
        tail.push("scale=w=trunc(iw/2)*2:h=trunc(ih/2)*2:flags=lanczos".to_string());
    }
    tail.push(format!("fps={fps}"));
    let tail = tail.join(",");

    if let [piece] = pieces {
        let whole = piece.start_s <= 0.01 && piece.end_s >= source.duration_s - MIN_PIECE_S;
        if !whole {
            if piece.start_s > 0.0 {
                push(&mut args, &["-ss", &piece.start_s.to_string()]);
            }
            push(
                &mut args,
                &["-t", &millis(piece.end_s - piece.start_s).to_string()],
            );
        }
        args.extend(["-i".into(), source.path.into()]);
        push(&mut args, &["-map", "0:v:0"]);
        for track in 0..tracks {
            push(&mut args, &["-map", &format!("0:a:{track}?")]);
        }
        push(&mut args, &["-vf", &tail]);
    } else {
        let mut chains = Vec::new();
        for (k, piece) in pieces.iter().enumerate() {
            let length = millis(piece.end_s - piece.start_s);
            args.extend(["-ss".into(), piece.start_s.to_string().into()]);
            args.extend(["-t".into(), length.to_string().into()]);
            args.extend(["-i".into(), source.path.into()]);
            chains.push(format!("[{k}:v]setpts=PTS-STARTPTS[v{k}]"));
            for t in 0..tracks {
                chains.push(format!("[{k}:a:{t}]asetpts=PTS-STARTPTS[a{k}_{t}]"));
            }
        }

        // concat wants each piece's video, then its audio tracks, grouped.
        let pads: String = (0..pieces.len())
            .map(|k| {
                let audio: String = (0..tracks).map(|t| format!("[a{k}_{t}]")).collect();
                format!("[v{k}]{audio}")
            })
            .collect();
        let audio_out: String = (0..tracks).map(|t| format!("[ac{t}]")).collect();
        chains.push(format!(
            "{pads}concat=n={}:v=1:a={tracks}[vc]{audio_out}",
            pieces.len()
        ));
        chains.push(format!("[vc]{tail}[vo]"));

        push(
            &mut args,
            &["-filter_complex", &chains.join(";"), "-map", "[vo]"],
        );
        for t in 0..tracks {
            push(&mut args, &["-map", &format!("[ac{t}]")]);
        }
    }

    push(
        &mut args,
        &[
            "-c:v",
            "libsvtav1",
            "-preset",
            "4",
            "-crf",
            &crf.to_string(),
        ],
    );
    push(
        &mut args,
        &[
            "-pix_fmt",
            "yuv420p10le",
            "-svtav1-params",
            "tune=0:keyint=300",
        ],
    );

    let colour = &source.colour;
    for (flag, tag) in [
        ("-colorspace", &colour.space),
        ("-color_primaries", &colour.primaries),
        ("-color_trc", &colour.trc),
    ] {
        if let Some(tag) = tag {
            push(&mut args, &[flag, tag]);
        }
    }

    if tracks > 0 {
        push(&mut args, &["-c:a", "libopus"]);
        for track in 0..tracks {
            let kbps = if track == 0 {
                MAIN_AUDIO_KBPS
            } else {
                EXTRA_AUDIO_KBPS
            };
            push(&mut args, &[&format!("-b:a:{track}"), &format!("{kbps}k")]);
        }
        push(&mut args, &["-mapping_family", "1"]);
    }

    push(
        &mut args,
        &[
            "-movflags",
            "+faststart",
            "-y",
            "-progress",
            "pipe:1",
            "-nostats",
        ],
    );
    args.push(output.into());

    args
}

fn push(args: &mut Vec<OsString>, items: &[&str]) {
    args.extend(items.iter().map(OsString::from));
}

fn millis(seconds: f64) -> f64 {
    (seconds * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece(start_s: f64, end_s: f64) -> Piece {
        Piece { start_s, end_s }
    }

    fn source(path: &Path) -> Source<'_> {
        Source {
            path,
            duration_s: 100.0,
            width: 2560,
            height: 1440,
            fps: 59.94,
            audio_tracks: 2,
            colour: Colour {
                space: Some("bt709".into()),
                primaries: Some("bt709".into()),
                trc: None,
            },
        }
    }

    fn text(args: Vec<OsString>) -> String {
        let parts: Vec<_> = args.iter().map(|a| a.to_string_lossy()).collect();
        parts.join(" ")
    }

    fn encode(source: &Source, pieces: &[Piece]) -> String {
        text(args(source, pieces, 46, Path::new("out.mp4")))
    }

    #[test]
    fn pieces_are_sorted_clamped_and_stripped_of_overlaps_and_slivers() {
        let messy = [
            piece(50.0, 60.0),
            piece(-5.0, 10.0),
            piece(8.0, 20.0),
            piece(30.0, 30.04),
            piece(90.0, 120.0),
        ];

        let kept = normalise(&messy, 100.0);

        assert_eq!(
            kept,
            [
                piece(0.0, 10.0),
                piece(10.0, 20.0),
                piece(50.0, 60.0),
                piece(90.0, 100.0)
            ]
        );
    }

    #[test]
    fn nothing_left_means_the_whole_video() {
        assert_eq!(normalise(&[piece(5.0, 5.01)], 100.0), [piece(0.0, 100.0)]);
        assert_eq!(normalise(&[], 100.0), [piece(0.0, 100.0)]);
    }

    #[test]
    fn one_piece_trims_at_the_input_and_clamps_the_frame_rate() {
        let path = Path::new("in.mp4");
        let line = encode(&source(path), &[piece(10.0, 25.5)]);

        let expected = "-ss 10 -t 15.5 -i in.mp4 -map 0:v:0 -map 0:a:0? -map 0:a:1? -vf fps=60 \
            -c:v libsvtav1 -preset 4 -crf 46 -pix_fmt yuv420p10le -svtav1-params tune=0:keyint=300 \
            -colorspace bt709 -color_primaries bt709 \
            -c:a libopus -b:a:0 96k -b:a:1 64k -mapping_family 1 \
            -movflags +faststart -y -progress pipe:1 -nostats out.mp4";
        assert_eq!(line, expected);
    }

    #[test]
    fn a_piece_spanning_the_whole_video_passes_untrimmed() {
        let path = Path::new("in.mp4");
        let line = encode(&source(path), &[piece(0.0, 100.0)]);

        assert!(line.starts_with("-i in.mp4 -map"), "{line}");
        assert!(!line.contains("-t "), "{line}");
    }

    #[test]
    fn a_slow_source_keeps_its_rate_and_an_odd_size_loses_a_pixel() {
        let path = Path::new("in.mp4");
        let mut odd = source(path);
        odd.fps = 30.0;
        odd.width = 1281;
        odd.audio_tracks = 0;

        let line = encode(&odd, &[piece(1.0, 5.0)]);

        assert!(
            line.contains("-vf scale=w=trunc(iw/2)*2:h=trunc(ih/2)*2:flags=lanczos,fps=30"),
            "{line}"
        );
        assert!(
            !line.contains("-map 0:a") && !line.contains("libopus"),
            "{line}"
        );
        assert_eq!(output_size(1281, 721), (1280, 720));
    }

    #[test]
    fn several_pieces_are_joined_before_the_frame_rate_is_set_once() {
        let path = Path::new("in.mp4");
        let line = encode(&source(path), &[piece(1.0, 3.0), piece(10.0, 14.5)]);

        let expected = "-ss 1 -t 2 -i in.mp4 -ss 10 -t 4.5 -i in.mp4 -filter_complex \
            [0:v]setpts=PTS-STARTPTS[v0];[0:a:0]asetpts=PTS-STARTPTS[a0_0];\
            [0:a:1]asetpts=PTS-STARTPTS[a0_1];[1:v]setpts=PTS-STARTPTS[v1];\
            [1:a:0]asetpts=PTS-STARTPTS[a1_0];[1:a:1]asetpts=PTS-STARTPTS[a1_1];\
            [v0][a0_0][a0_1][v1][a1_0][a1_1]concat=n=2:v=1:a=2[vc][ac0][ac1];[vc]fps=60[vo] \
            -map [vo] -map [ac0] -map [ac1] -c:v libsvtav1";
        assert!(line.starts_with(expected), "{line}");
        assert_eq!(line.matches("fps=").count(), 1);
    }
}
