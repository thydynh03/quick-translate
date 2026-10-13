//! Subtitle files: every line of an .srt or .vtt file is translated ahead of
//! time and written next to the original, so the player shows the translation
//! with no delay at all.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::engine;
use crate::settings::Settings;

/// Lines sent in one request: enough for the engine to see the conversation,
/// small enough for every engine to accept.
const BATCH_LINES: usize = 40;
const BATCH_CHARS: usize = 1800;

#[derive(Debug, PartialEq)]
pub enum Part {
    /// Kept as it is: the WEBVTT header, notes, cues without text.
    Raw(String),
    /// `head` is everything up to and including the timing line.
    Cue { head: String, text: String },
}

/// Removes markup such as `<i>` and `{\an8}`, and joins the lines of a cue.
fn plain(lines: &[&str]) -> String {
    let mut out = String::new();
    let mut closing = None;
    for c in lines.join(" ").chars() {
        match (closing, c) {
            (Some(end), _) if c == end => closing = None,
            (Some(_), _) => {}
            (None, '<') => closing = Some('>'),
            (None, '{') => closing = Some('}'),
            (None, _) => out.push(c),
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn parse(source: &str) -> Vec<Part> {
    let source = source.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    // A line holding only spaces separates cues like an empty one.
    let lines: Vec<&str> = source.lines().collect();
    lines
        .split(|line| line.trim().is_empty())
        .filter(|block| !block.is_empty())
        .map(|block| block.join("\n"))
        .map(|block| {
            let block = block.as_str();
            let lines: Vec<&str> = block.lines().collect();
            let Some(timing) = lines.iter().position(|line| line.contains("-->")) else {
                return Part::Raw(block.to_string());
            };
            let text = plain(&lines[timing + 1..]);
            if text.is_empty() {
                return Part::Raw(block.to_string());
            }
            Part::Cue { head: lines[..=timing].join("\n"), text }
        })
        .collect()
}

/// The file again, with the text of the n-th cue replaced by `texts[n]`.
pub fn render(parts: &[Part], texts: &[String]) -> String {
    let mut texts = texts.iter();
    let mut out = String::new();
    for part in parts {
        match part {
            Part::Raw(block) => out.push_str(block),
            Part::Cue { head, text } => {
                out.push_str(head);
                out.push('\n');
                out.push_str(texts.next().unwrap_or(text));
            }
        }
        out.push_str("\n\n");
    }
    out
}

/// Consecutive runs of `texts` that fit one request each.
pub fn batches(texts: &[String]) -> Vec<std::ops::Range<usize>> {
    let mut out = Vec::new();
    let (mut start, mut chars) = (0, 0);
    for (index, text) in texts.iter().enumerate() {
        let length = text.chars().count() + 1;
        if index > start && (index - start >= BATCH_LINES || chars + length > BATCH_CHARS) {
            out.push(start..index);
            (start, chars) = (index, 0);
        }
        chars += length;
    }
    if start < texts.len() {
        out.push(start..texts.len());
    }
    out
}

/// "film.srt" -> "film.vi.srt"; a language already in the name is replaced
/// ("film.en.srt" -> "film.vi.srt").
pub fn output_path(source: &Path, lang: &str) -> PathBuf {
    let extension = source.extension().and_then(|e| e.to_str()).unwrap_or("srt");
    let stem = source.file_stem().and_then(|s| s.to_str()).unwrap_or("subtitles");
    let base = match stem.rsplit_once('.') {
        Some((base, code)) if (2..=3).contains(&code.len()) && code.chars().all(|c| c.is_ascii_alphabetic()) => base,
        _ => stem,
    };
    source.with_file_name(format!("{base}.{lang}.{extension}"))
}

/// One request for the whole batch, so the engine sees the lines around each
/// line; line by line when it does not answer with as many lines.
async fn translate_batch(settings: &Settings, models: &Path, lines: &[String]) -> anyhow::Result<Vec<String>> {
    let joined = lines.join("\n");
    let mut whole = engine::translate_text(settings, models, &joined).await;
    if whole.is_err() {
        // Most likely the service asking us to slow down.
        tokio::time::sleep(Duration::from_secs(3)).await;
        whole = engine::translate_text(settings, models, &joined).await;
    }
    let whole = whole?;
    let parts: Vec<String> = whole.text.split('\n').map(|part| part.trim().to_string()).collect();
    if parts.len() == lines.len() && parts.iter().all(|part| !part.is_empty()) {
        return Ok(parts);
    }
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let translated = engine::translate_text(settings, models, line).await;
        out.push(translated.map(|found| found.text).unwrap_or_else(|_| line.clone()));
    }
    Ok(out)
}

/// Subtitle files are UTF-8 or, from older Windows tools, UTF-16 with a byte order mark.
pub fn decode(bytes: &[u8]) -> String {
    let wide = |order: fn([u8; 2]) -> u16| {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|pair| order([pair[0], pair[1]])).collect();
        String::from_utf16_lossy(&units)
    };
    match bytes {
        [0xff, 0xfe, ..] => wide(u16::from_le_bytes),
        [0xfe, 0xff, ..] => wide(u16::from_be_bytes),
        _ => String::from_utf8_lossy(bytes).into_owned(),
    }
}

#[derive(Serialize, Clone)]
struct Progress {
    path: String,
    done: usize,
    total: usize,
}

/// Translates the subtitle file at `path` into the user's language and
/// returns where the translation was written.
pub async fn translate_file(app: &AppHandle, path: &Path) -> anyhow::Result<PathBuf> {
    let settings = crate::effective(app);
    let output = output_path(path, &settings.primary_lang);
    // "film.vi.srt" would be written over itself.
    anyhow::ensure!(output != path, "File này đã mang tên của bản dịch ({})", settings.primary_lang);
    let bytes = tokio::fs::read(path).await?;
    let parts = parse(&decode(&bytes));
    let texts: Vec<String> =
        parts.iter().filter_map(|part| if let Part::Cue { text, .. } = part { Some(text.clone()) } else { None }).collect();
    anyhow::ensure!(!texts.is_empty(), "Không tìm thấy dòng phụ đề nào trong file");

    let models = engine::offline::models_dir(app);
    let shown = path.display().to_string();
    let mut translated: Vec<String> = Vec::with_capacity(texts.len());
    let _ = app.emit("subtitle-progress", Progress { path: shown.clone(), done: 0, total: texts.len() });
    for range in batches(&texts) {
        translated.extend(translate_batch(&settings, &models, &texts[range]).await?);
        let _ = app.emit("subtitle-progress", Progress { path: shown.clone(), done: translated.len(), total: texts.len() });
        // An online service must not take the run for an attack.
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    tokio::fs::write(&output, render(&parts, &translated)).await?;
    crate::log(format!("subtitles: {} lines -> {}", translated.len(), output.display()));
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\u{feff}1\r\n00:00:01,000 --> 00:00:03,000\r\n<i>I never said that</i>\r\nto anyone.\r\n\r\n2\r\n00:00:04,000 --> 00:00:05,000\r\n{\\an8}Really?\r\n\r\n3\r\n00:00:06,000 --> 00:00:07,000\r\n\r\n";

    #[test]
    fn reads_cues_and_drops_markup() {
        let parts = parse(SAMPLE);
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[0], Part::Cue { head: "1\n00:00:01,000 --> 00:00:03,000".into(), text: "I never said that to anyone.".into() });
        assert_eq!(parts[1], Part::Cue { head: "2\n00:00:04,000 --> 00:00:05,000".into(), text: "Really?".into() });
        assert!(matches!(parts[2], Part::Raw(_)));
    }

    #[test]
    fn writes_the_translation_under_the_same_timing() {
        let parts = parse(SAMPLE);
        let out = render(&parts, &["Tôi chưa từng nói vậy với ai.".to_string(), "Thật sao?".to_string()]);
        assert!(out.starts_with("1\n00:00:01,000 --> 00:00:03,000\nTôi chưa từng nói vậy với ai.\n\n2\n"));
        assert!(out.contains("00:00:04,000 --> 00:00:05,000\nThật sao?\n\n3\n00:00:06,000 --> 00:00:07,000\n\n"));
    }

    #[test]
    fn reads_utf16_and_cues_split_by_blank_looking_lines() {
        let mut bytes = vec![0xff, 0xfe];
        bytes.extend("1\n00:00:01,000 --> 00:00:02,000\nHi\n \n2\n00:00:03,000 --> 00:00:04,000\nBye".encode_utf16().flat_map(u16::to_le_bytes));
        let parts = parse(&decode(&bytes));
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[1], Part::Cue { head: "2\n00:00:03,000 --> 00:00:04,000".into(), text: "Bye".into() });
    }

    #[test]
    fn keeps_the_header_of_a_vtt_file() {
        let parts = parse("WEBVTT\n\n00:01.000 --> 00:02.000\nHello");
        assert_eq!(parts[0], Part::Raw("WEBVTT".into()));
        assert!(render(&parts, &["Xin chào".to_string()]).starts_with("WEBVTT\n\n00:01.000 --> 00:02.000\nXin chào"));
    }

    #[test]
    fn batches_cover_every_line_once() {
        let texts: Vec<String> = (0..95).map(|n| format!("line {n}")).collect();
        let ranges = batches(&texts);
        assert_eq!(ranges, vec![0..40, 40..80, 80..95]);
        let long: Vec<String> = (0..3).map(|_| "x".repeat(1000)).collect();
        assert_eq!(batches(&long), vec![0..1, 1..2, 2..3]);
        assert!(batches(&[]).is_empty());
    }

    #[test]
    fn names_the_output_after_the_language() {
        assert_eq!(output_path(Path::new("a/film.srt"), "vi"), Path::new("a/film.vi.srt"));
        assert_eq!(output_path(Path::new("a/film.en.srt"), "vi"), Path::new("a/film.vi.srt"));
        assert_eq!(output_path(Path::new("a/film.2019.vtt"), "vi"), Path::new("a/film.2019.vi.vtt"));
    }

    /// A real engine answers a batch with one line per line.
    #[test]
    #[ignore]
    fn a_batch_comes_back_line_for_line() {
        let lines: Vec<String> = ["Where were you last night?", "I told you, I was at home.", "Nobody saw you there.", "Then ask my sister.", "We already did."]
            .iter()
            .map(|line| line.to_string())
            .collect();
        let settings = Settings::default();
        let started = std::time::Instant::now();
        let out = tauri::async_runtime::block_on(translate_batch(&settings, Path::new("."), &lines)).unwrap();
        println!("{:?} {out:?}", started.elapsed());
        assert_eq!(out.len(), lines.len());
        assert!(out.iter().zip(&lines).all(|(translated, line)| translated != line));
    }
}
