use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime};
use regex::Regex;
use std::sync::LazyLock;

const GREAT: &str = "\u{2b50}";
const BEST: &str = "\u{1f308}";

// ShadowPlay's own shape: "Base Profile 2024.02.20 - 23.46.01.02.DVR.mp4"; the last number is a counter.
static SHADOWPLAY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(\d{4})\.(\d{2})\.(\d{2})\s*-\s*(\d{2})\.(\d{2})\.(\d{2})(?:\.(\d{1,4}))?")
        .unwrap()
});

// The older "2023-10-14 10-52-28" shape has no counter.
static OLDER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(\d{4})-(\d{2})-(\d{2})[ _](\d{2})-(\d{2})-(\d{2})").unwrap());

// When a video was recorded, from its file name, else its modified time; and the counter
// ShadowPlay gives each recording of a session ("00" when the name has none).
pub fn recorded(file_name: &str, modified_ms: i64) -> (NaiveDateTime, String) {
    for pattern in [&SHADOWPLAY, &OLDER] {
        let Some(found) = pattern.captures(file_name) else {
            continue;
        };
        let number = |i: usize| found[i].parse::<u32>().unwrap();
        let date = NaiveDate::from_ymd_opt(number(1) as i32, number(2), number(3));
        let Some(at) = date.and_then(|d| d.and_hms_opt(number(4), number(5), number(6))) else {
            continue;
        };
        let counter = found.get(7).map_or("00", |c| c.as_str());

        return (at, counter.to_owned());
    }

    let modified = DateTime::from_timestamp_millis(modified_ms).unwrap_or_default();
    (
        modified.with_timezone(&Local).naive_local(),
        "00".to_owned(),
    )
}

// Text for a file name: no character Windows refuses, no control characters, one space between words.
pub fn sanitize(text: &str) -> String {
    let swapped: String = text
        .chars()
        .filter(|c| !c.is_control())
        .map(|c| if "\\/:*?\"<>|".contains(c) { '-' } else { c })
        .collect();
    let spaced = swapped.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = spaced.trim_matches(|c: char| c == '.' || c.is_whitespace());

    trimmed.chars().take(120).collect()
}

// "<Game> - YYYY.MM.dd - hh.mm.ss.rv - <Title>[ ⭐][🌈].mp4": the video's recorded time moved on to where
// the clip starts, so the clip reads as the moment it came from and sorts beside its original.
pub fn file_name(
    game: &str,
    title: &str,
    great: bool,
    best: bool,
    video_name: &str,
    modified_ms: i64,
    start_s: f64,
) -> String {
    let (at, counter) = recorded(video_name, modified_ms);
    let at = at + Duration::seconds(start_s.floor() as i64);
    let stamp = at.format("%Y.%m.%d - %H.%M.%S");

    let mut labels = String::new();
    if great {
        labels.push_str(GREAT);
    }
    if best {
        labels.push_str(BEST);
    }
    let labels = if labels.is_empty() {
        labels
    } else {
        format!(" {labels}")
    };

    format!(
        "{} - {stamp}.{counter} - {}{labels}.mp4",
        sanitize(game),
        sanitize(title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(text: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn a_shadowplay_name_gives_its_time_and_counter() {
        let name = "Base Profile 2024.02.20 - 23.46.01.02.DVR.mp4";
        assert_eq!(recorded(name, 0), (at("2024-02-20 23:46:01"), "02".into()));
    }

    #[test]
    fn the_older_name_has_no_counter() {
        let name = "Desktop 2023-10-14 10-52-28.mp4";
        assert_eq!(recorded(name, 0), (at("2023-10-14 10:52:28"), "00".into()));
    }

    #[test]
    fn an_unnamed_recording_takes_its_modified_time() {
        let (_, counter) = recorded("clip.mp4", 1_700_000_000_000);
        assert_eq!(counter, "00");
    }

    #[test]
    fn a_name_with_an_impossible_date_falls_through() {
        let (_, counter) = recorded("x 2024.13.40 - 25.00.00.07.mp4", 0);
        assert_eq!(counter, "00");
    }

    #[test]
    fn the_file_name_starts_the_stamp_at_the_clips_first_second() {
        let name = file_name(
            "PEAK",
            "Big jump",
            false,
            false,
            "PEAK 2024.02.20 - 23.59.30.02.DVR.mp4",
            0,
            45.9,
        );
        assert_eq!(name, "PEAK - 2024.02.21 - 00.00.15.02 - Big jump.mp4");
    }

    #[test]
    fn great_and_best_end_the_name_in_that_order() {
        let name = |great, best| {
            file_name(
                "PEAK",
                "Jump",
                great,
                best,
                "PEAK 2024.02.20 - 10.00.00.01.mp4",
                0,
                0.0,
            )
        };
        assert_eq!(
            name(true, true),
            "PEAK - 2024.02.20 - 10.00.00.01 - Jump \u{2b50}\u{1f308}.mp4"
        );
        assert_eq!(
            name(false, true),
            "PEAK - 2024.02.20 - 10.00.00.01 - Jump \u{1f308}.mp4"
        );
        assert_eq!(
            name(false, false),
            "PEAK - 2024.02.20 - 10.00.00.01 - Jump.mp4"
        );
    }

    #[test]
    fn game_and_title_lose_what_a_file_name_cannot_hold() {
        assert_eq!(sanitize("Counter-Strike: GO"), "Counter-Strike- GO");
        assert_eq!(sanitize("  ..a\t/  b?.. "), "a- b-");
        assert_eq!(sanitize(&"x".repeat(200)).chars().count(), 120);
    }
}
