//! BailanCraft: interface text wraps Thai rows between words.
//!
//! egui's default [`SimpleLineBreaker`] doesn't know where Thai words end (they have no spaces
//! between them), so a long Thai label or message wrapped between any two grapheme clusters,
//! cutting words in half. [`ThaiLineBreaker`] keeps every break the default finds and adds the
//! ends of Thai words from the ICU4X line segmenter's dictionary (the same segmentation the
//! edit crate uses for Thai written into PDFs).

use egui::text::{LineBreak, LineBreaker, SimpleLineBreaker};

/// [`SimpleLineBreaker`] plus breaks between Thai words.
#[derive(Clone, Copy, Debug, Default)]
pub struct ThaiLineBreaker;

impl LineBreaker for ThaiLineBreaker {
    fn line_breaks(&self, text: &str, language: Option<&str>, out: &mut Vec<LineBreak>) {
        SimpleLineBreaker.line_breaks(text, language, out);
        out.extend(thai_word_breaks(text).into_iter().map(LineBreak::soft));
    }
}

fn is_thai(c: char) -> bool {
    matches!(u32::from(c), 0x0E00..=0x0E7F)
}

/// Byte offsets inside `text` where one Thai word ends and the next row could begin: next to a
/// Thai character, never before a space, closing punctuation, mai yamok (ๆ) or paiyan noi (ฯ).
fn thai_word_breaks(text: &str) -> Vec<usize> {
    if !text.chars().any(is_thai) {
        return Vec::new();
    }
    let segmenter = icu_segmenter::LineSegmenter::new_dictionary(icu_segmenter::options::LineBreakOptions::default());
    segmenter
        .segment_str(text)
        .filter(|&at| at > 0 && at < text.len() && text.is_char_boundary(at))
        .filter(|&at| {
            let before = text.get(..at).and_then(|s| s.chars().next_back());
            let after = text.get(at..).and_then(|s| s.chars().next());
            match (before, after) {
                (Some(b), Some(a)) => {
                    (is_thai(b) || is_thai(a))
                        && !a.is_whitespace()
                        && !matches!(a, 'ๆ' | 'ฯ' | '.' | ',' | ':' | ';' | '!' | '?' | ')' | ']' | '}' | '”' | '’' | '%')
                        && !matches!(b, '(' | '[' | '{' | '“' | '‘' | '฿' | '$')
                }
                _ => false,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn breaks(text: &str) -> Vec<usize> {
        let mut out = Vec::new();
        ThaiLineBreaker.line_breaks(text, Some("th"), &mut out);
        out.into_iter().map(|b| b.offset).collect()
    }

    #[test]
    fn thai_words_get_breaks_between_them() {
        let text = "กล่องข้อความ";
        let at = breaks(text);
        let word2 = text.find("ข้อความ").unwrap();
        assert!(at.contains(&word2), "{at:?}");
        // Not inside กล่อง (after กล่) nor inside ข้อความ (after ข้อ).
        assert!(!at.contains(&"กล่".len()), "{at:?}");
        assert!(!at.contains(&(word2 + "ข้อ".len())), "{at:?}");
    }

    #[test]
    fn text_without_thai_keeps_the_default_breaks() {
        for text in ["Open the file", "日本語の文章", "", "a-b/c"] {
            let mut default = Vec::new();
            SimpleLineBreaker.line_breaks(text, None, &mut default);
            assert_eq!(breaks(text), default.into_iter().map(|b| b.offset).collect::<Vec<_>>(), "{text:?}");
        }
    }

    #[test]
    fn no_break_before_spaces_marks_or_punctuation() {
        for text in ["เด็กๆ เล่นกัน", "กรุงเทพฯ เป็นเมืองหลวง", "ราคา ฿1,250.00 บาท.", "(ภาษาไทย)", "น้ำ ค่ำ กำ"]
        {
            for b in thai_word_breaks(text) {
                let after = text[b..].chars().next().unwrap();
                assert!(!after.is_whitespace() && !matches!(after, 'ๆ' | 'ฯ' | '.' | ')'), "{text:?} at {b}");
                // Never in front of a vowel or tone mark: those belong to the letter before.
                assert!(!matches!(after, '\u{0E31}' | '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}'), "{text:?} at {b}");
            }
        }
    }
}
