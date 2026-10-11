//! Cheap local language guess, enough to choose the translation direction
//! without a network round trip.

const VI_MARKS: &str = "ăâđêôơưàảãáạằẳẵắặầẩẫấậèẻẽéẹềểễếệìỉĩíịòỏõóọồổỗốộờởỡớợùủũúụừửữứựỳỷỹýỵ";
/// Letters Vietnamese shares with French, Spanish or Portuguese.
const SHARED_MARKS: &str = "âêôàãáèéìíòõóùúý";

/// Returns an ISO 639-1 code when the script makes the language obvious.
pub fn guess(text: &str) -> Option<String> {
    let mut letters = 0usize;
    let mut vi = 0usize;
    let mut vi_only = 0usize;
    let mut kana = 0usize;
    let mut han = 0usize;
    let mut hangul = 0usize;
    let mut cyrillic = 0usize;
    let mut thai = 0usize;

    for c in text.chars().flat_map(|c| c.to_lowercase()) {
        if !c.is_alphabetic() {
            continue;
        }
        letters += 1;
        match c as u32 {
            0x3040..=0x30FF => kana += 1,
            0x4E00..=0x9FFF => han += 1,
            0xAC00..=0xD7AF => hangul += 1,
            0x0400..=0x04FF => cyrillic += 1,
            0x0E00..=0x0E7F => thai += 1,
            _ if VI_MARKS.contains(c) => {
                vi += 1;
                if !SHARED_MARKS.contains(c) {
                    vi_only += 1;
                }
            }
            _ => {}
        }
    }
    if letters == 0 {
        return None;
    }
    let share = |n: usize| n * 10 >= letters * 3;
    if kana > 0 {
        Some("ja".into())
    } else if share(hangul) {
        Some("ko".into())
    } else if share(han) {
        Some("zh".into())
    } else if share(cyrillic) {
        Some("ru".into())
    } else if share(thai) {
        Some("th".into())
    } else if vi_only > 0 && vi * 12 >= letters {
        // Vietnamese: roughly one marked letter per word, and at least one
        // letter no other Latin-script language uses.
        Some("vi".into())
    } else {
        None
    }
}

/// One word (or a short hyphenated compound): show dictionary senses.
pub fn is_single_word(text: &str) -> bool {
    let t = text.trim();
    !t.is_empty() && t.chars().count() <= 40 && t.split_whitespace().count() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guesses_by_script() {
        assert_eq!(guess("Xin chào, tôi là người Việt").as_deref(), Some("vi"));
        assert_eq!(guess("Hello, how are you today?"), None);
        assert_eq!(guess("こんにちは世界").as_deref(), Some("ja"));
        assert_eq!(guess("你好世界").as_deref(), Some("zh"));
        assert_eq!(guess("안녕하세요").as_deref(), Some("ko"));
        assert_eq!(guess("Привет мир").as_deref(), Some("ru"));
        assert_eq!(guess("1234 !!"), None);
    }

    #[test]
    fn french_accents_are_not_vietnamese() {
        assert_eq!(guess("Voilà un café très agréable près de la mer"), None);
    }

    #[test]
    fn single_word() {
        assert!(is_single_word(" serendipity "));
        assert!(!is_single_word("two words"));
        assert!(!is_single_word(""));
    }
}
