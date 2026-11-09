//! BailanCraft: the Unicode text of Thai as older Thai fonts and Microsoft Word write it.
//!
//! Thai fonts of the Windows era (Cordia New, Angsana New, Browallia New, …) keep shifted forms of
//! the marks and of ญ/ฐ without their tails at private-use code points U+F700–U+F71A, and their
//! `cmap` lists them there. A PDF whose `/ToUnicode` was made from that `cmap` then reads those
//! glyphs as private-use characters, which no font shows: copy, search and the text editor saw
//! "ช าระ" for "ชำระ". Word also draws ำ (SARA AM) as two glyphs, NIKHAHIT then SARA AA, so the
//! text reads back as "ชํา" — it looks the same, but searching for "ชำระ" does not find it.
//!
//! [`standard`] maps the private-use forms to the characters they show; [`compose_sara_am`] puts
//! NIKHAHIT (with any tone mark next to it) and SARA AA back together as SARA AM.
use std::borrow::Cow;

const NIKHAHIT: char = '\u{0E4D}';
const SARA_AA: char = '\u{0E32}';
const SARA_AM: char = '\u{0E33}';

/// The Thai character a private-use Thai glyph form shows, or `c` itself.
pub fn standard(c: char) -> char {
    let mapped = match u32::from(c) {
        0xF700 => 0x0E10,
        0xF701 => 0x0E34,
        0xF702 => 0x0E35,
        0xF703 => 0x0E36,
        0xF704 => 0x0E37,
        0xF705 | 0xF70A | 0xF713 => 0x0E48,
        0xF706 | 0xF70B | 0xF714 => 0x0E49,
        0xF707 | 0xF70C | 0xF715 => 0x0E4A,
        0xF708 | 0xF70D | 0xF716 => 0x0E4B,
        0xF709 | 0xF70E | 0xF717 => 0x0E4C,
        0xF70F => 0x0E0D,
        0xF710 => 0x0E31,
        0xF711 => 0x0E4D,
        0xF712 => 0x0E47,
        0xF718 => 0x0E38,
        0xF719 => 0x0E39,
        0xF71A => 0x0E3A,
        _ => return c,
    };
    char::from_u32(mapped).unwrap_or(c)
}

/// Whether `text` has a private-use Thai form in it.
fn has_private_use(text: &str) -> bool {
    text.chars().any(|c| matches!(u32::from(c), 0xF700..=0xF71A))
}

/// `text` with its private-use Thai forms mapped to the characters they show.
pub fn standard_text(text: String) -> String {
    if has_private_use(&text) { text.chars().map(standard).collect() } else { text }
}

/// Thai tone marks (MAI EK … MAI CHATTAWA).
pub fn is_tone(c: char) -> bool {
    matches!(c, '\u{0E48}'..='\u{0E4B}')
}

/// `text` with NIKHAHIT, an optional tone mark, then SARA AA written as the tone mark and SARA AM
/// (and NIKHAHIT straight before SARA AA as SARA AM).
pub fn compose_sara_am(text: &str) -> Cow<'_, str> {
    if !text.contains(NIKHAHIT) || !text.contains(SARA_AA) {
        return Cow::Borrowed(text);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(&c) = chars.get(i) {
        if c == NIKHAHIT {
            match (chars.get(i + 1).copied(), chars.get(i + 2).copied()) {
                (Some(SARA_AA), _) => {
                    out.push(SARA_AM);
                    i += 2;
                    continue;
                }
                (Some(tone), Some(SARA_AA)) if is_tone(tone) => {
                    out.push(tone);
                    out.push(SARA_AM);
                    i += 3;
                    continue;
                }
                _ => {}
            }
        }
        out.push(c);
        i += 1;
    }
    Cow::Owned(out)
}

/// [`standard_text`] then [`compose_sara_am`].
pub fn normalize(text: &str) -> Cow<'_, str> {
    if has_private_use(text) {
        let standard = standard_text(text.to_owned());
        Cow::Owned(compose_sara_am(&standard).into_owned())
    } else {
        compose_sara_am(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn private_use_thai_forms_read_as_the_characters_they_show() {
        assert_eq!(standard_text("ช\u{F711}าระ".into()), "ชําระ");
        assert_eq!(standard_text("ป\u{F70B}".into()), "ป้");
        assert_eq!(standard_text("\u{F700}าน \u{F70F}".into()), "ฐาน ญ");
        assert_eq!(standard('a'), 'a');
        assert_eq!(standard('\u{F71B}'), '\u{F71B}', "past the Thai forms");
    }

    #[test]
    fn nikhahit_and_sara_aa_read_as_sara_am() {
        assert_eq!(compose_sara_am("ชําระ"), "ชำระ");
        // Word puts NIKHAHIT before the tone mark: น ํ ้ า is น้ำ.
        assert_eq!(compose_sara_am("น\u{0E4D}\u{0E49}\u{0E32}"), "น้ำ");
        assert_eq!(compose_sara_am("น้ำ"), "น้ำ");
        assert_eq!(compose_sara_am("\u{0E4D}"), "\u{0E4D}", "a lone NIKHAHIT stays");
        assert_eq!(compose_sara_am("กํ ก"), "กํ ก");
        assert!(matches!(compose_sara_am("plain"), Cow::Borrowed(_)));
        assert_eq!(normalize("จ\u{F711}านวน"), "จำนวน");
    }
}
