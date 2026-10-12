//! Cutting the sentence around a position out of a paragraph, for hover
//! translation and for explaining a selected word in its context.

/// Characters that end a sentence when followed by a space or the end.
const ENDS: [char; 6] = ['.', '!', '?', '…', '。', '؟'];
/// Closing quotes and brackets that stay with the sentence they end.
const CLOSERS: [char; 6] = ['"', '\'', ')', ']', '”', '’'];

/// Byte ranges of the sentences in `text`, trimmed.
fn sentences(text: &str) -> Vec<(usize, usize)> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut out = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        let boundary = if c == '\n' {
            Some(at)
        } else if ENDS.contains(&c) {
            // Keep runs like "?!" or "..." and closing quotes together.
            let mut j = i + 1;
            while j < chars.len() && (ENDS.contains(&chars[j].1) || CLOSERS.contains(&chars[j].1)) {
                j += 1;
            }
            // "3.5", "e.g.x" or a URL: a period followed by a letter or digit is not an end.
            let next_is_space = j == chars.len() || chars[j].1.is_whitespace();
            if next_is_space && !is_abbreviation(&text[start..at], c) {
                i = j - 1;
                Some(if j == chars.len() { text.len() } else { chars[j].0 })
            } else {
                None
            }
        } else {
            None
        };
        if let Some(end) = boundary {
            push_trimmed(text, start, end, &mut out);
            start = end;
        }
        i += 1;
    }
    push_trimmed(text, start, text.len(), &mut out);
    out
}

fn push_trimmed(text: &str, start: usize, end: usize, out: &mut Vec<(usize, usize)>) {
    let slice = &text[start..end];
    let lead = slice.len() - slice.trim_start().len();
    let trail = slice.len() - slice.trim_end().len();
    if start + lead < end - trail {
        out.push((start + lead, end - trail));
    }
}

/// "Mr." / "Dr." / "e.g." / a single initial: a period that does not end the sentence.
fn is_abbreviation(before: &str, end: char) -> bool {
    if end != '.' {
        return false;
    }
    let last = before.split_whitespace().last().unwrap_or("").to_lowercase();
    const SHORT: [&str; 14] = ["mr", "mrs", "ms", "dr", "prof", "sr", "jr", "st", "vs", "etc", "e.g", "i.e", "no", "fig"];
    let original = before.split_whitespace().last().unwrap_or("");
    SHORT.contains(&last.as_str()) || (original.chars().count() == 1 && original.chars().all(char::is_uppercase))
}

/// The sentence of `paragraph` that contains the character at `char_offset`.
/// An offset past the end gives the last sentence.
pub fn around(paragraph: &str, char_offset: usize) -> Option<String> {
    let byte = paragraph.char_indices().nth(char_offset).map_or(paragraph.len(), |(b, _)| b);
    let all = sentences(paragraph);
    all.iter()
        .find(|(s, e)| byte < *e && byte >= *s)
        .or_else(|| all.iter().find(|(_, e)| byte <= *e))
        .or(all.last())
        .map(|(s, e)| paragraph[*s..*e].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: &str = "The meeting moved to Thursday. Please confirm whether you can attend! Thanks, David";

    #[test]
    fn picks_the_sentence_under_the_offset() {
        assert_eq!(around(TEXT, 3).unwrap(), "The meeting moved to Thursday.");
        let attend = TEXT.find("attend").unwrap();
        assert_eq!(around(TEXT, attend).unwrap(), "Please confirm whether you can attend!");
        assert_eq!(around(TEXT, TEXT.len() - 2).unwrap(), "Thanks, David");
    }

    #[test]
    fn decimals_and_abbreviations_do_not_split() {
        let text = "Mr. Smith paid 3.5 dollars, e.g. for tea. Then he left.";
        assert_eq!(around(text, 5).unwrap(), "Mr. Smith paid 3.5 dollars, e.g. for tea.");
    }

    #[test]
    fn lines_and_quotes() {
        let text = "Title line\nShe said \"it is done.\" Then J. Doe left.";
        assert_eq!(around(text, 2).unwrap(), "Title line");
        assert_eq!(around(text, 13).unwrap(), "She said \"it is done.\"");
        assert_eq!(around(text, text.len() - 3).unwrap(), "Then J. Doe left.");
    }

    #[test]
    fn vietnamese_text_uses_char_offsets() {
        let text = "Cuộc họp đã dời. Bạn có tham dự không?";
        let offset = text.chars().position(|c| c == 'B').unwrap();
        assert_eq!(around(text, offset).unwrap(), "Bạn có tham dự không?");
    }
}
