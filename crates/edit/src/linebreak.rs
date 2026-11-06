//! Where a row of rewrapped text may end besides a space: between two characters when either is
//! written without spaces between words (Chinese and Japanese ideographs and kana), following the
//! simple cases of Unicode line breaking (UAX #14, class ID). The row must not then begin with a
//! character that closes or follows (closing brackets and quotes, commas and full stops, small
//! kana, the prolonged sound mark, iteration marks) or end with one that opens (opening brackets
//! and quotes, currency signs before an amount): the line-start and line-end prohibitions of
//! JIS X 4051 ("kinsoku"), which UAX #14 gives as classes CL, CP, EX, IS, NS and OP, PR.
//!
//! Korean is written with spaces between words and keeps breaking only there. Thai (and Lao,
//! Khmer, Myanmar) is written without spaces between words: BailanCraft finds its words with the
//! ICU4X line segmenter's dictionary ([`opportunities`]), so a row ends between two words and
//! never inside one.

/// Writing with no spaces between words, where a row may end between two characters (UAX #14 ID).
fn ideographic(c: char) -> bool {
    matches!(c,
        '\u{2e80}'..='\u{2fdf}' // CJK radicals, Kangxi radicals
        | '\u{3000}'..='\u{303f}' // CJK symbols and punctuation
        | '\u{3040}'..='\u{30ff}' // hiragana, katakana
        | '\u{3190}'..='\u{31ff}' // kanbun, CJK strokes, katakana phonetic extensions
        | '\u{3400}'..='\u{4dbf}' // CJK extension A
        | '\u{4e00}'..='\u{9fff}' // CJK unified ideographs
        | '\u{f900}'..='\u{faff}' // CJK compatibility ideographs
        | '\u{fe30}'..='\u{fe4f}' // CJK compatibility forms
        | '\u{ff00}'..='\u{ffef}' // half-width and full-width forms
        | '\u{20000}'..='\u{3fffd}' // supplementary ideographic planes
    )
}

/// Characters that must not begin a row: closing brackets and quotes, and the punctuation, small
/// kana and marks that belong to the character before them (UAX #14 CL, CP, EX, IS, NS). The
/// ideographic space breaks after itself, not before (BA): it is Japanese spacing and indentation.
fn no_start(c: char) -> bool {
    matches!(c,
        ')' | ']' | '}' | '»' | '’' | '”' | '!' | '?' | ',' | '.' | ':' | ';' | '%' | '‰' | '°' | '℃'
        | '‐' | '–' | '〜' | '‼' | '⁇' | '⁈' | '⁉' | '・' | '\u{3000}'
        | '、' | '。' | '〃' | '〆' | '々' | '〻' | '〕' | '〉' | '》' | '」' | '』' | '】' | '〙' | '〗' | '〟' | '〞'
        | 'ぁ' | 'ぃ' | 'ぅ' | 'ぇ' | 'ぉ' | 'っ' | 'ゃ' | 'ゅ' | 'ょ' | 'ゎ' | 'ゕ' | 'ゖ'
        | 'ァ' | 'ィ' | 'ゥ' | 'ェ' | 'ォ' | 'ッ' | 'ャ' | 'ュ' | 'ョ' | 'ヮ' | 'ヵ' | 'ヶ'
        | '゛' | '゜' | 'ゝ' | 'ゞ' | '゠' | 'ー' | 'ヽ' | 'ヾ'
        | '\u{31f0}'..='\u{31ff}' // small katakana for Ainu
        | '！' | '％' | '）' | '，' | '．' | '：' | '；' | '？' | '］' | '｝' | '｠' | '～'
        | '｡' | '｣' | '､' | '･' | 'ｰ' | 'ﾞ' | 'ﾟ'
        | '\u{ff67}'..='\u{ff6f}' // small half-width katakana
    )
}

/// Characters that must not end a row: opening brackets and quotes, and currency signs written
/// before an amount (UAX #14 OP, PR).
fn no_end(c: char) -> bool {
    matches!(
        c,
        '(' | '['
            | '{'
            | '«'
            | '‘'
            | '“'
            | '$'
            | '£'
            | '¥'
            | '€'
            | '〈'
            | '《'
            | '「'
            | '『'
            | '【'
            | '〔'
            | '〖'
            | '〘'
            | '〝'
            | '（'
            | '［'
            | '｛'
            | '｟'
            | '｢'
            | '＄'
            | '￡'
            | '￥'
    )
}

/// Whether a row may end between `before` and `after`, neither of them a space.
pub(crate) fn between(before: char, after: char) -> bool {
    (ideographic(before) || ideographic(after)) && !no_start(after) && !no_end(before)
}

/// Thai letters, vowels, tone marks and digits (U+0E00–U+0E7F).
pub(crate) fn is_thai(c: char) -> bool {
    matches!(u32::from(c), 0x0E00..=0x0E7F)
}

/// Byte offsets in `text` where a row may end, by Unicode line breaking (UAX #14) with dictionary
/// segmentation for scripts written without spaces between words (Thai, Lao, Khmer, Myanmar).
/// Each offset is where the next row would begin, on a char boundary; 0 and `text.len()` are
/// left out. Spaces stay at the end of the row before the break.
pub(crate) fn opportunities(text: &str) -> Vec<usize> {
    let segmenter = icu_segmenter::LineSegmenter::new_dictionary(icu_segmenter::options::LineBreakOptions::default());
    // The dictionary marks every end of a Thai word, also before a space or a full stop, where
    // UAX #14 itself never breaks: keep only breaks the row-start and row-end rules allow.
    segmenter
        .segment_str(text)
        .filter(|&at| at > 0 && at < text.len() && text.is_char_boundary(at))
        .filter(|&at| {
            let before = text.get(..at).and_then(|s| s.chars().next_back());
            let after = text.get(at..).and_then(|s| s.chars().next());
            match (before, after) {
                (Some(b), Some(a)) => !a.is_whitespace() && !no_start(a) && !thai_no_start(a) && !no_end(b),
                _ => false,
            }
        })
        .collect()
}

/// Thai characters that belong to the word before them and must not begin a row: mai yamok
/// (ๆ, repetition) and paiyan noi (ฯ, abbreviation).
fn thai_no_start(c: char) -> bool {
    matches!(c, 'ๆ' | 'ฯ')
}

/// `word` (text without spaces) cut where a row may end inside it; the pieces join with nothing.
pub(crate) fn pieces(word: &str) -> Vec<&str> {
    if word.chars().any(is_thai) {
        let mut out = Vec::new();
        let mut start = 0;
        for at in opportunities(word) {
            out.extend(word.get(start..at));
            start = at;
        }
        out.extend(word.get(start..).filter(|rest| !rest.is_empty()));
        return out;
    }
    let mut out = Vec::new();
    let mut start = 0;
    let mut prev: Option<char> = None;
    for (at, c) in word.char_indices() {
        if let Some(before) = prev
            && between(before, c)
        {
            out.extend(word.get(start..at));
            start = at;
        }
        prev = Some(c);
    }
    out.extend(word.get(start..).filter(|rest| !rest.is_empty()));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ideographs_and_kana_break_between_characters_except_around_kinsoku_marks() {
        assert_eq!(pieces("日本語の文章"), ["日", "本", "語", "の", "文", "章"]);
        // Closing punctuation and small kana stay with the character before them; an opening
        // bracket stays with the one after it.
        assert_eq!(pieces("ちょっと、「はい」。"), ["ちょっ", "と、", "「は", "い」。"]);
        assert_eq!(pieces("コーヒー"), ["コー", "ヒー"]);
        assert_eq!(pieces("（例）です"), ["（例）", "で", "す"]);
        // Latin text inside a Japanese sentence breaks before and after it, never inside it.
        assert_eq!(pieces("PDFを編集"), ["PDF", "を", "編", "集"]);
        assert_eq!(pieces("価格は￥100です"), ["価", "格", "は", "￥100", "で", "す"]);
        // Words of space-separated writing are never cut.
        assert_eq!(pieces("wrapping"), ["wrapping"]);
        assert_eq!(pieces("한국어"), ["한국어"]);
        assert_eq!(pieces(""), Vec::<&str>::new());
        assert!(!between('a', 'b') && between('字', 'a') && !between('字', '。') && !between('「', '字'));
    }

    #[test]
    fn thai_breaks_between_words_never_inside_one() {
        // "กล่องข้อความ" (text box) is two words: กล่อง + ข้อความ. Cutting after "กล่" splits a
        // word and strands its vowel and final consonant.
        let cut = pieces("กล่องข้อความ");
        assert_eq!(cut.concat(), "กล่องข้อความ");
        assert!(cut.contains(&"กล่อง"), "{cut:?}");
        assert!(!cut.iter().any(|p| *p == "กล่"), "{cut:?}");
        // Marks never start a piece: every piece begins with a base letter.
        let sentence = "ประโยคยาวที่ไม่มีเว้นวรรคเพื่อทดสอบการตัดคำภาษาไทยเมื่อข้อความยาวเกินความกว้างของกล่องข้อความ";
        let words = pieces(sentence);
        assert_eq!(words.concat(), sentence);
        assert!(words.len() > 10, "a long sentence has many words: {words:?}");
        for w in &words {
            let first = w.chars().next().unwrap_or(' ');
            assert!(!matches!(first, '\u{0E31}' | '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}' | 'ะ' | 'า' | 'ำ'), "{w:?} starts with a mark");
        }
        for word in ["ภาษาไทย", "ข้อความ", "ความกว้าง"] {
            assert!(words.iter().any(|w| w.contains(word) || word.contains(w)), "{word} in {words:?}");
        }
        assert!(opportunities("").is_empty() && opportunities("ก").is_empty());
    }

    #[test]
    fn opportunities_stay_on_char_boundaries_and_inside_the_text() {
        for text in ["น้ำ ค่ำ กำ", "ราคา ฿1,250.00 บาท", "ใบเสนอราคา Quotation No. 2569/0042", "ฤๅษี ฦๅ ๆ ฯลฯ", "a\u{0E48}\u{0E49}"]
        {
            for at in opportunities(text) {
                assert!(at > 0 && at < text.len() && text.is_char_boundary(at), "{text:?} at {at}");
            }
        }
        // No break inside "฿1,250.00" or between a word and the full stop after it.
        let text = "ราคา ฿1,250.00 บาท.";
        let at = opportunities(text);
        let inside = |s: &str| text.find(s).map(|i| (i + 1..i + s.len()).any(|b| at.contains(&b))).unwrap_or(true);
        assert!(!inside("฿1,250.00"), "{at:?}");
        assert!(!inside("บาท."), "{at:?}");
        // Never before a space; mai yamok and paiyan noi stay with the word before them.
        assert!(at.iter().all(|&b| !text[b..].starts_with(' ')), "{at:?}");
        for text in ["เด็กๆ เล่นกัน", "กรุงเทพฯ เป็นเมืองหลวง"] {
            let at = opportunities(text);
            assert!(at.iter().all(|&b| !text[b..].starts_with('ๆ') && !text[b..].starts_with('ฯ')), "{text}: {at:?}");
        }
    }
}
