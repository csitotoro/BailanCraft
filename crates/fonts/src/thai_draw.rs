//! BailanCraft: Thai in generated appearance streams (form field values, Fill & Sign text, text
//! box comments), which used to draw every Thai character as `?` in Helvetica.
//!
//! Appearance builders are pure (`&Document`) and can't add the indirect objects a font needs,
//! so drawing Thai takes two steps:
//!
//! 1. The builder draws each line with [`show_line`]: Latin runs in its own font, Thai shaped by
//!    [`crate::shape_thai`] into clusters (a letter with its vowels and tone marks), each cluster
//!    one code of a Type 3 font named `BCTh0`, `BCTh1`, … ([`font_name`]). It records the lines
//!    it drew, in order, under [`LINES_KEY`] in the stream dictionary ([`record_lines`]).
//! 2. Whoever stores the stream calls [`materialize`] with the document: the same lines give the
//!    same clusters in the same order, so the same codes, and the Type 3 fonts (glyph outlines
//!    from the craft-fonts Thai face, a ToUnicode map for copy and search) are added and named
//!    in the stream's `/Resources`.
//!
//! The Type 3 writer follows the one Add Text uses (`pdfcraft-edit`, `added/draw.rs`), kept
//! separate so this fork's change stays out of upstream's file.

use std::collections::HashMap;

use pdfcraft_cos::{Dict, Document, ObjRef, Object, PdfString, Stream};

use crate::{GlyphError, GlyphOutline, ShapedCluster, ShapedScript, literal, shape_thai, shaped_glyph, win_ansi_encodable};

/// The private key on an appearance stream listing the lines drawn with [`show_line`], as text
/// strings in drawing order. [`materialize`] reads and removes it.
pub const LINES_KEY: &[u8] = b"PCThaiLines";

/// Most clusters in one Type 3 font: codes are single bytes, 1–240.
const PER_FONT: usize = 240;

/// The resource name of the `i`-th Thai Type 3 font of an appearance.
pub fn font_name(i: usize) -> String {
    format!("{FONT_PREFIX}{i}")
}

/// Thai letters, vowels, tone marks and digits (U+0E00–U+0E7F).
pub fn is_thai(c: char) -> bool {
    matches!(u32::from(c), 0x0E00..=0x0E7F)
}

/// Whether `text` has Thai in it.
pub fn has_thai(text: &str) -> bool {
    text.chars().any(is_thai)
}

/// BailanCraft: the bundled Thai face as a static font, for embedding as a whole font program
/// (Edit text). craft-fonts ships Noto Sans Thai as a variable font, which the embedder refuses;
/// its `glyf` outlines and metrics already are the default instance, so dropping the variation
/// tables leaves that instance as an ordinary TrueType font. `None` without the face or when its
/// table directory can't be read.
pub fn static_thai_face() -> Option<&'static [u8]> {
    static FACE: std::sync::OnceLock<Option<Vec<u8>>> = std::sync::OnceLock::new();
    FACE.get_or_init(|| crate::document_thai_font().and_then(|f| drop_variations(f.bytes))).as_deref()
}

/// `bytes` (one sfnt, not a collection) without its variation tables, or `None` if malformed.
fn drop_variations(bytes: &[u8]) -> Option<Vec<u8>> {
    const VARIATION: [&[u8; 4]; 8] = [b"fvar", b"gvar", b"avar", b"HVAR", b"VVAR", b"MVAR", b"STAT", b"cvar"];
    if bytes.get(0..4)? == b"ttcf" {
        return None;
    }
    crate::system_faces::rebuild(bytes, 0, &VARIATION)
}

/// Whether this build can draw Thai (it has the craft-fonts Thai face).
pub fn available() -> bool {
    crate::document_thai_font().is_some()
}

/// Vowels and tone marks written above or below the letter before them: never on their own.
fn is_mark(c: char) -> bool {
    matches!(c, '\u{0E31}' | '\u{0E34}'..='\u{0E3A}' | '\u{0E47}'..='\u{0E4E}')
}

/// `line` cut into runs of Thai and of everything else, in order.
fn runs(line: &str) -> Vec<(bool, &str)> {
    let mut out: Vec<(bool, &str)> = Vec::new();
    let mut start = 0;
    let mut current: Option<bool> = None;
    for (at, c) in line.char_indices() {
        let thai = is_thai(c);
        if current.is_some_and(|t| t != thai) {
            if let (Some(t), Some(run)) = (current, line.get(start..at)) {
                out.push((t, run));
            }
            start = at;
        }
        current = Some(thai);
    }
    if let (Some(t), Some(run)) = (current, line.get(start..)) {
        if !run.is_empty() {
            out.push((t, run));
        }
    }
    out
}

/// The width of `line` at `size` points: Thai runs by their shaped advances, the rest by `latin`.
/// Without the Thai face (or for text it can't shape) the whole line is measured by `latin`.
pub fn width(line: &str, size: f64, latin: impl Fn(&str, f64) -> f64) -> f64 {
    let mut total = 0.0;
    for (thai, run) in runs(line) {
        if thai {
            match shape_thai(run) {
                Ok(clusters) => total += clusters.iter().map(|c| c.advance).sum::<f64>() * size,
                Err(_) => return latin(line, size),
            }
        } else {
            total += latin(run, size);
        }
    }
    total
}

/// The first character of `text` that would still come out as `?`: neither Thai the face has nor
/// a character `latin_ok` says the Latin font holds. Line breaks don't count.
pub fn first_undrawable(text: &str, latin_ok: impl Fn(char) -> bool) -> Option<char> {
    text.chars().find(|&c| !matches!(c, '\n' | '\r') && if is_thai(c) { !crate::thai_has(c) } else { !latin_ok(c) })
}

/// [`first_undrawable`] with the standard fonts (WinAnsiEncoding) for the Latin runs.
pub fn first_undrawable_standard(text: &str) -> Option<char> {
    first_undrawable(text, win_ansi_encodable)
}

/// Where a row of Thai text may end: UAX #14 with the ICU4X dictionary for Thai words, never
/// before a space, closing punctuation, mai yamok (ๆ) or paiyan noi (ฯ), nor after an opening
/// bracket or a currency sign. Byte offsets of where the next row starts.
fn breaks(text: &str) -> Vec<usize> {
    let segmenter = icu_segmenter::LineSegmenter::new_dictionary(icu_segmenter::options::LineBreakOptions::default());
    segmenter
        .segment_str(text)
        .filter(|&at| at > 0 && at < text.len() && text.is_char_boundary(at))
        .filter(|&at| {
            let before = text.get(..at).and_then(|s| s.chars().next_back());
            let after = text.get(at..).and_then(|s| s.chars().next());
            match (before, after) {
                (Some(b), Some(a)) => {
                    !a.is_whitespace()
                        && !is_mark(a)
                        && !matches!(a, 'ๆ' | 'ฯ' | '.' | ',' | ':' | ';' | '!' | '?' | ')' | ']' | '}' | '”' | '’' | '%')
                        && !matches!(b, '(' | '[' | '{' | '“' | '‘' | '฿' | '$')
                }
                _ => false,
            }
        })
        .collect()
}

/// Greedy line breaking within `width` points for text with Thai: paragraphs split on line
/// breaks (CRLF is one), rows end between words ([`breaks`]: Thai words from a dictionary, after
/// spaces). A piece wider than a row is cut between letters, never before a vowel or tone mark.
/// Spaces at the end of a row are dropped.
pub fn wrap(text: &str, size: f64, row: f64, measure: impl Fn(&str, f64) -> f64) -> Vec<String> {
    let mut lines = Vec::new();
    for para in text.replace("\r\n", "\n").split(['\n', '\r']) {
        let mut cuts = breaks(para);
        cuts.push(para.len());
        let mut line = String::new();
        let mut from = 0usize;
        for to in cuts {
            let Some(piece) = para.get(from..to) else { continue };
            from = to;
            let candidate = format!("{line}{piece}");
            if measure(candidate.trim_end_matches(' '), size) <= row {
                line = candidate;
                continue;
            }
            if !line.is_empty() {
                lines.push(line.trim_end_matches(' ').to_owned());
                line = String::new();
            }
            // The piece starts the next row, cut between letters if it is wider than a row.
            for unit in letters(piece) {
                let next = format!("{line}{unit}");
                if !line.is_empty() && measure(next.trim_end_matches(' '), size) > row {
                    lines.push(line.trim_end_matches(' ').to_owned());
                    line = unit.to_owned();
                } else {
                    line = next;
                }
            }
        }
        lines.push(line.trim_end_matches(' ').to_owned());
    }
    lines
}

/// `text` cut into letters with the vowels and tone marks above or below them (and Sara Am),
/// the smallest pieces a row may be cut into.
pub fn letters(text: &str) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    let mut start = 0;
    for (at, c) in text.char_indices() {
        if at > 0 && !is_mark(c) && c != 'ำ' {
            out.extend(text.get(start..at));
            start = at;
        }
    }
    out.extend(text.get(start..).filter(|s| !s.is_empty()));
    out
}

/// What makes two clusters the same Type 3 glyph: text, glyphs, offsets and advance (bit for bit).
type Key = (String, Vec<(u32, u64, u64)>, u64);

fn key(c: &ShapedCluster) -> Key {
    (c.text.clone(), c.glyphs.iter().map(|(id, [x, y])| (*id, x.to_bits(), y.to_bits())).collect(), c.advance.to_bits())
}

/// The distinct Thai clusters of an appearance, in the order they were first drawn.
#[derive(Default)]
pub struct Glyphs {
    clusters: Vec<ShapedCluster>,
    index: HashMap<Key, usize>,
}

impl Glyphs {
    pub fn new() -> Self {
        Self::default()
    }

    /// The font index and code of `c`, registering it when new.
    fn code(&mut self, c: &ShapedCluster) -> (usize, u8) {
        let k = *self.index.entry(key(c)).or_insert_with(|| {
            self.clusters.push(c.clone());
            self.clusters.len() - 1
        });
        // Codes start at 1; k % 240 + 1 is at most 240.
        (k / PER_FONT, u8::try_from(k % PER_FONT + 1).unwrap_or(1))
    }

    pub fn is_empty(&self) -> bool {
        self.clusters.is_empty()
    }
}

/// Text-showing operators for `line` from the current text position (after a `Tm` or `Td`):
/// Latin runs in `latin_font` at `size`, encoded with `encode`; Thai clusters in the Thai
/// Type 3 fonts, each `Tj` moving the pen by the cluster's advance. Ends with `latin_font`
/// selected again, as the builder left it. Thai the face can't shape is drawn with `encode`
/// (as `?`), like the rest of the builder's undrawable text.
pub fn show_line(glyphs: &mut Glyphs, line: &str, size: f64, latin_font: &str, encode: impl Fn(&str) -> Vec<u8>) -> Vec<u8> {
    show_line_as(glyphs, line, size, latin_font, encode, FONT_PREFIX)
}

/// The resource-name prefix of an appearance's own Thai fonts ([`font_name`]).
const FONT_PREFIX: &str = "BCTh";

/// [`show_line`] with the Thai fonts named `{prefix}0`, `{prefix}1`, … : for text drawn straight
/// into page content, where several marks share the page's resources and each needs its own names.
pub fn show_line_as(glyphs: &mut Glyphs, line: &str, size: f64, latin_font: &str, encode: impl Fn(&str) -> Vec<u8>, prefix: &str) -> Vec<u8> {
    let font_name = |i: usize| format!("{prefix}{i}");
    let mut out = Vec::new();
    let mut font: Option<usize> = None;
    let latin_tf = format!("/{latin_font} {} Tf ", num(size));
    for (thai, run) in runs(line) {
        let shaped = if thai { shape_thai(run).ok() } else { None };
        match shaped {
            Some(clusters) => {
                for c in &clusters {
                    let (f, code) = glyphs.code(c);
                    if font != Some(f) {
                        out.extend(format!("/{} {} Tf ", font_name(f), num(size)).bytes());
                        font = Some(f);
                    }
                    out.extend(literal(&[code]));
                    out.extend_from_slice(b" Tj ");
                }
            }
            None => {
                if font.is_some() {
                    out.extend(latin_tf.bytes());
                    font = None;
                }
                out.extend(literal(&encode(run)));
                out.extend_from_slice(b" Tj ");
            }
        }
    }
    if font.is_some() {
        out.extend(latin_tf.bytes());
    }
    out.push(b'\n');
    out
}

/// Record the lines drawn with [`show_line`] (in drawing order) on the stream dictionary.
pub fn record_lines(d: &mut Dict, lines: &[String]) {
    if lines.is_empty() {
        return;
    }
    d.set(LINES_KEY.to_vec(), Object::Array(lines.iter().map(|l| Object::String(PdfString::text(l))).collect()));
}

/// Add the Thai Type 3 fonts an appearance stream drawn with [`show_line`] needs, and name them
/// in its `/Resources /Font`. Does nothing to a stream without [`LINES_KEY`]. On error (no Thai
/// face) the stream is left as it was, key and all, and nothing is added to `doc`.
pub fn materialize(doc: &mut Document, stream: &mut Stream) -> Result<(), GlyphError> {
    let Some(Object::Array(lines)) = stream.dict.get(LINES_KEY).cloned() else { return Ok(()) };
    let mut glyphs = Glyphs::new();
    for line in lines.iter().filter_map(Object::as_string).map(PdfString::to_text) {
        // Codes are assigned exactly as `show_line` assigned them.
        let _ = show_line(&mut glyphs, &line, 1.0, "F", |_| Vec::new());
    }
    let fonts = write_fonts(doc, &glyphs, FONT_PREFIX)?;
    let mut res = match stream.dict.get(b"Resources").map(|o| doc.resolve(o)) {
        Some(r) => r.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    let mut font_dict = match res.get(b"Font").map(|o| doc.resolve(o)) {
        Some(f) => f.as_dict().cloned().unwrap_or_default(),
        None => Dict::new(),
    };
    for (name, f) in fonts {
        font_dict.set(name.into_bytes(), Object::Ref(f));
    }
    res.set(b"Font".to_vec(), Object::Dict(font_dict));
    stream.dict.set(b"Resources".to_vec(), Object::Dict(res));
    stream.dict.remove(LINES_KEY);
    Ok(())
}

/// Add the Thai Type 3 fonts for every cluster in `glyphs` to `doc`, named `{prefix}0`, … as
/// [`show_line_as`] named them. Every outline is read before anything is written, so on error
/// (no Thai face) nothing is added.
pub fn write_fonts(doc: &mut Document, glyphs: &Glyphs, prefix: &str) -> Result<Vec<(String, ObjRef)>, GlyphError> {
    let mut outlines: HashMap<u32, GlyphOutline> = HashMap::new();
    for c in &glyphs.clusters {
        for (id, _) in &c.glyphs {
            if !outlines.contains_key(id) {
                outlines.insert(*id, shaped_glyph(ShapedScript::Thai, *id)?);
            }
        }
    }
    Ok(glyphs.clusters.chunks(PER_FONT).enumerate().map(|(i, chunk)| (format!("{prefix}{i}"), type3(doc, chunk, &outlines))).collect())
}

/// Format a number for content streams (up to three decimals).
fn num(v: f64) -> String {
    let s = format!("{:.3}", if v.abs() < 5e-4 || !v.is_finite() { 0.0 } else { v });
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// A Type 3 font with `clusters` as codes 1…, each drawn from the Thai face's glyph `outlines`,
/// with a ToUnicode map giving each code its cluster's text.
fn type3(doc: &mut Document, clusters: &[ShapedCluster], outlines: &HashMap<u32, GlyphOutline>) -> ObjRef {
    let mut charprocs = Dict::new();
    let mut differences = vec![Object::Int(1)];
    let mut widths = Vec::with_capacity(clusters.len());
    let mut bbox = [0.0f64; 4];
    let mut to_unicode = Vec::new();
    for (i, c) in clusters.iter().enumerate() {
        let code = i + 1;
        // Glyph space is 1000 units per em.
        let mut paths = Vec::new();
        let mut ink: Option<[f64; 4]> = None;
        for (id, [dx, dy]) in &c.glyphs {
            let Some(g) = outlines.get(id) else { continue };
            for contour in &g.contours {
                for (j, p) in contour.iter().enumerate() {
                    paths.extend(format!("{} {} {}\n", num((p[0] + dx) * 1000.0), num((p[1] + dy) * 1000.0), if j == 0 { "m" } else { "l" }).bytes());
                }
                paths.extend_from_slice(b"h\n");
                let b = [g.bbox[0] + dx, g.bbox[1] + dy, g.bbox[2] + dx, g.bbox[3] + dy];
                ink = Some(ink.map_or(b, |a| [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]));
            }
        }
        // Rounded outwards so no ink is clipped.
        let [x0, y0, x1, y1] =
            ink.map_or([0.0; 4], |b| [(b[0] * 1000.0).floor(), (b[1] * 1000.0).floor(), (b[2] * 1000.0).ceil(), (b[3] * 1000.0).ceil()]);
        bbox = [bbox[0].min(x0), bbox[1].min(y0), bbox[2].max(x1), bbox[3].max(y1)];
        let mut proc = format!("{} 0 {} {} {} {} d1\n", num(c.advance * 1000.0), num(x0), num(y0), num(x1), num(y1)).into_bytes();
        if !paths.is_empty() {
            proc.extend(paths);
            proc.extend_from_slice(b"f\n");
        }
        let glyph = format!("g{code:02X}");
        charprocs.set(glyph.clone().into_bytes(), Object::Ref(doc.add(Object::Stream(Stream::flate(Dict::new(), &proc)))));
        differences.push(Object::name(&glyph));
        widths.push(Object::Real((c.advance * 1000.0).round()));
        if !c.text.is_empty() {
            let hex: String = c.text.encode_utf16().map(|u| format!("{u:04X}")).collect();
            to_unicode.push(format!("<{code:02X}> <{hex}>\n"));
        }
    }
    let mut cmap = String::from(
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CMapType 2 def\n1 begincodespacerange\n<01> <FF>\nendcodespacerange\n",
    );
    // A CMap block holds at most 100 entries.
    for block in to_unicode.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n{}endbfchar\n", block.len(), block.concat()));
    }
    cmap.push_str("endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n");
    let cmap = doc.add(Object::Stream(Stream::flate(Dict::new(), cmap.as_bytes())));
    let family = ShapedScript::Thai.font().map_or("Thai", |f| f.family);
    let mut descriptor = Dict::new();
    descriptor.set(b"Type".to_vec(), Object::name("FontDescriptor"));
    descriptor.set(b"FontName".to_vec(), Object::name(&family.replace(' ', "")));
    descriptor.set(b"FontFamily".to_vec(), Object::String(PdfString::literal(family.as_bytes().to_vec())));
    descriptor.set(b"Flags".to_vec(), Object::Int(4));
    descriptor.set(b"ItalicAngle".to_vec(), Object::Int(0));
    let mut encoding = Dict::new();
    encoding.set(b"Type".to_vec(), Object::name("Encoding"));
    encoding.set(b"Differences".to_vec(), Object::Array(differences));
    let mut font = Dict::new();
    font.set(b"Type".to_vec(), Object::name("Font"));
    font.set(b"Subtype".to_vec(), Object::name("Type3"));
    font.set(b"FontDescriptor".to_vec(), Object::Ref(doc.add(Object::Dict(descriptor))));
    font.set(b"FontBBox".to_vec(), Object::Array(bbox.iter().map(|v| Object::Real(*v)).collect()));
    font.set(
        b"FontMatrix".to_vec(),
        Object::Array(vec![Object::Real(0.001), Object::Int(0), Object::Int(0), Object::Real(0.001), Object::Int(0), Object::Int(0)]),
    );
    font.set(b"FirstChar".to_vec(), Object::Int(1));
    font.set(b"LastChar".to_vec(), Object::Int(i64::try_from(clusters.len()).unwrap_or(1)));
    font.set(b"Widths".to_vec(), Object::Array(widths));
    font.set(b"Encoding".to_vec(), Object::Dict(encoding));
    font.set(b"CharProcs".to_vec(), Object::Dict(charprocs));
    font.set(b"ToUnicode".to_vec(), Object::Ref(cmap));
    doc.add(Object::Dict(font))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_static_thai_face_embeds() {
        let Some(face) = static_thai_face() else { return };
        crate::embedding::validate(face).unwrap();
        assert!(crate::embedding::covers(face, "ข้อความ น้ำ ปู่ ABC 123 %"));
        assert!(crate::source_shaping::shape(face, "ปู่น้ำ").is_ok());
        assert_eq!(drop_variations(b"ttcf...."), None);
        assert_eq!(drop_variations(b""), None);
    }

    #[test]
    fn runs_split_thai_from_the_rest() {
        assert_eq!(runs("ราคา ฿1,250 บาท"), [(true, "ราคา"), (false, " "), (true, "฿"), (false, "1,250 "), (true, "บาท")]);
        assert_eq!(runs("abc"), [(false, "abc")]);
        assert!(runs("").is_empty());
    }

    #[test]
    fn letters_keep_marks_with_their_letter() {
        assert_eq!(letters("ปู่น้ำ"), ["ปู่", "น้ำ"]);
        assert_eq!(letters("ab"), ["a", "b"]);
        assert!(letters("").is_empty());
    }

    #[test]
    fn wrap_ends_rows_between_thai_words() {
        // One "point" per character, so widths are character counts.
        let measure = |s: &str, _: f64| s.chars().count() as f64;
        let text = "ภาษาไทยเมื่อข้อความยาวเกินความกว้างของกล่องข้อความ";
        for row in [8.0, 10.0, 14.0, 20.0] {
            let lines = wrap(text, 1.0, row, measure);
            assert_eq!(lines.concat(), text, "{row}: {lines:?}");
            for pair in lines.windows(2) {
                assert!(!(pair[0].ends_with("กล่") && pair[1].starts_with("อง")), "{row}: {lines:?}");
                assert!(!(pair[0].ends_with("ข้อ") && pair[1].starts_with("ความ")), "{row}: {lines:?}");
            }
            assert!(lines.iter().all(|l| l.chars().next().is_none_or(|c| !is_mark(c))), "{row}: {lines:?}");
        }
        // Spaces and line breaks.
        assert_eq!(wrap("ชื่อ นามสกุล", 1.0, 7.0, measure).first().map(String::as_str), Some("ชื่อ"));
        assert_eq!(wrap("ก\nข", 1.0, 10.0, measure), ["ก", "ข"]);
        assert_eq!(wrap("", 1.0, 10.0, measure), [""]);
        // A word wider than the row is cut between letters, marks staying with theirs.
        let cut = wrap("ผู้ใหญ่บ้าน", 1.0, 3.0, measure);
        assert_eq!(cut.concat(), "ผู้ใหญ่บ้าน");
        assert!(cut.iter().all(|l| l.chars().next().is_none_or(|c| !is_mark(c))), "{cut:?}");
    }

    #[test]
    fn undrawable_checks_thai_and_latin() {
        assert_eq!(first_undrawable("abc", |_| true), None);
        assert_eq!(first_undrawable("ab日", win_ansi_encodable), Some('日'));
        if available() {
            assert_eq!(first_undrawable_standard("ชื่อ John ๑๒๓"), None);
        } else {
            assert_eq!(first_undrawable_standard("ชื่อ"), Some('ช'));
        }
    }

    #[test]
    fn show_line_and_materialize_agree_on_codes() {
        if !available() {
            return;
        }
        let mut glyphs = Glyphs::new();
        let lines = ["นาย สมชาย ใจดี".to_string(), "Tel 081 น้ำ".to_string()];
        let mut ops = Vec::new();
        for l in &lines {
            ops.extend(show_line(&mut glyphs, l, 12.0, "Helv", crate::win_ansi));
        }
        let ops = String::from_utf8_lossy(&ops);
        assert!(ops.contains("/BCTh0 12 Tf") && ops.contains("/Helv 12 Tf") && ops.contains("(Tel 081 ) Tj"), "{ops}");
        let mut stream = Stream::flate(Dict::new(), ops.as_bytes());
        record_lines(&mut stream.dict, &lines);
        let mut doc = Document::new_empty();
        materialize(&mut doc, &mut stream).unwrap();
        assert!(stream.dict.get(LINES_KEY).is_none());
        let res = stream.dict.get(b"Resources").and_then(Object::as_dict).unwrap();
        let fonts = res.get(b"Font").and_then(Object::as_dict).unwrap();
        let r = fonts.get(b"BCTh0").and_then(Object::as_ref).unwrap();
        let font = doc.get(r);
        let font = font.as_dict().unwrap();
        assert_eq!(font.name(b"Subtype"), Some(&b"Type3"[..]));
        // One code per distinct cluster, and as many as show_line used.
        let last = font.get(b"LastChar").and_then(Object::as_int).unwrap();
        assert_eq!(usize::try_from(last).unwrap(), glyphs.clusters.len());
        // Without LINES_KEY nothing happens.
        let mut plain = Stream::flate(Dict::new(), b"BT ET");
        materialize(&mut doc, &mut plain).unwrap();
        assert!(plain.dict.get(b"Resources").is_none());
    }
}
