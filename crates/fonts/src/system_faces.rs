//! BailanCraft: faces installed on this computer (Cordia New, Angsana New, TH Sarabun New, …).
//!
//! Thai documents are mostly written in fonts Windows or the user installed, and a PDF keeps only
//! a subset of them. Editing such a PDF with the installed face keeps its look, as Word does when
//! it embeds the same font. A face is offered only when its own embedding permission (OS/2
//! `fsType`) allows editable embedding ([`crate::embedding::validate`]).
//!
//! The index reads only each file's table directory and `name` table; a face's program is read
//! when it is used. Faces of a collection (`.ttc`) are taken out into a standalone font.
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// One installed face: the file, its index in a collection, and its names.
#[derive(Clone, Debug, PartialEq)]
pub struct SystemFace {
    pub path: PathBuf,
    pub index: u32,
    pub family: String,
    pub full_name: String,
    pub postscript: String,
    /// Whether its embedding permission lets a PDF keep it for editing.
    pub embeddable: bool,
}

/// Most font files read for the index, and the largest file looked at.
const MAX_FILES: usize = 4096;
const MAX_FILE: u64 = 64 << 20;
/// Most faces one collection lists.
const MAX_COLLECTION: u32 = 64;

/// The folders fonts are installed in.
pub fn directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(windows) {
        let windir = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SystemRoot")).map_or_else(|| PathBuf::from(r"C:\Windows"), PathBuf::from);
        dirs.push(windir.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft").join("Windows").join("Fonts"));
        }
    } else if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/Library/Fonts"));
        if let Some(home) = std::env::var_os("HOME") {
            dirs.push(PathBuf::from(home).join("Library/Fonts"));
        }
    } else {
        dirs.extend([PathBuf::from("/usr/share/fonts"), PathBuf::from("/usr/local/share/fonts")]);
        if let Some(home) = std::env::var_os("HOME") {
            dirs.extend([PathBuf::from(&home).join(".local/share/fonts"), PathBuf::from(home).join(".fonts")]);
        }
    }
    dirs
}

/// Every installed face, read once.
pub fn installed() -> &'static [SystemFace] {
    static INDEX: OnceLock<Vec<SystemFace>> = OnceLock::new();
    INDEX.get_or_init(|| scan(&directories()))
}

/// The faces under `roots` (two folder levels deep), in file order.
pub fn scan(roots: &[PathBuf]) -> Vec<SystemFace> {
    let mut files = Vec::new();
    let mut pending: Vec<(PathBuf, usize)> = roots.iter().map(|r| (r.clone(), 0)).collect();
    while let Some((dir, depth)) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        for entry in entries.flatten().take(MAX_FILES) {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() && depth < 2 {
                pending.push((path, depth + 1));
            } else if kind.is_file() && is_font_file(&path) && files.len() < MAX_FILES {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut faces = Vec::new();
    for path in files {
        faces.extend(faces_in(&path).unwrap_or_default());
    }
    faces
}

fn is_font_file(path: &Path) -> bool {
    path.extension().and_then(std::ffi::OsStr::to_str).is_some_and(|e| ["ttf", "otf", "ttc"].iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// The installed faces whose PostScript or full name is `name` (a PDF `/BaseFont`, with or without
/// its subset tag; "CordiaNew,Bold" and "CordiaNew-Bold" are the same face).
pub fn matching(name: &str) -> Vec<&'static SystemFace> {
    let key = crate::embedding::face_key(name);
    if key.is_empty() {
        return Vec::new();
    }
    installed()
        .iter()
        .filter(|f| f.embeddable)
        .filter(|f| crate::embedding::face_key(&f.postscript) == key || crate::embedding::face_key(&f.full_name) == key)
        .collect()
}

/// [`load`], kept for the next call (the editor asks once per character).
pub fn program(face: &SystemFace) -> Option<std::sync::Arc<Vec<u8>>> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    type Programs = Mutex<HashMap<(PathBuf, u32), Option<Arc<Vec<u8>>>>>;
    static PROGRAMS: OnceLock<Programs> = OnceLock::new();
    let programs = PROGRAMS.get_or_init(Default::default);
    let key = (face.path.clone(), face.index);
    if let Ok(cache) = programs.lock()
        && let Some(found) = cache.get(&key)
    {
        return found.clone();
    }
    let loaded = load(face).map(Arc::new);
    if let Ok(mut cache) = programs.lock() {
        if cache.len() >= 64 {
            cache.clear();
        }
        cache.insert(key, loaded.clone());
    }
    loaded
}

/// The face's program as a standalone font, when it may be embedded for editing.
pub fn load(face: &SystemFace) -> Option<Vec<u8>> {
    let bytes = read_limited(&face.path)?;
    let standalone = standalone(&bytes, face.index)?;
    crate::embedding::validate(&standalone).is_ok().then_some(standalone)
}

/// Face `index` of the font file at `path` as a standalone font (at most 64 MiB is read).
pub fn read_face(path: &Path, index: u32) -> Option<Vec<u8>> {
    standalone(&read_limited(path)?, index)
}

fn read_limited(path: &Path) -> Option<Vec<u8>> {
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_FILE {
        return None;
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE).read_to_end(&mut bytes).ok()?;
    Some(bytes)
}

/// Face `index` of a font file: the file itself for a single font, the face taken out of a
/// collection (`ttcf`) otherwise.
pub fn standalone(bytes: &[u8], index: u32) -> Option<Vec<u8>> {
    if bytes.get(0..4)? != b"ttcf" {
        return (index == 0).then(|| bytes.to_vec());
    }
    let count = be32(bytes, 8)?;
    if index >= count {
        return None;
    }
    let at = usize::try_from(be32(bytes, 12usize.checked_add(usize::try_from(index).ok()?.checked_mul(4)?)?)?).ok()?;
    rebuild(bytes, at, &[])
}

/// The font whose table directory starts at `at` in `bytes`, written as a standalone font
/// without the tables `drop` names.
pub(crate) fn rebuild(bytes: &[u8], at: usize, drop: &[&[u8; 4]]) -> Option<Vec<u8>> {
    let version = bytes.get(at..at.checked_add(4)?)?;
    let count = usize::from(be16(bytes, at.checked_add(4)?)?);
    let mut tables: Vec<([u8; 4], u32, &[u8])> = Vec::with_capacity(count);
    for i in 0..count {
        let record = at.checked_add(12)?.checked_add(i.checked_mul(16)?)?;
        let tag: [u8; 4] = bytes.get(record..record.checked_add(4)?)?.try_into().ok()?;
        let checksum = be32(bytes, record.checked_add(4)?)?;
        let offset = usize::try_from(be32(bytes, record.checked_add(8)?)?).ok()?;
        let length = usize::try_from(be32(bytes, record.checked_add(12)?)?).ok()?;
        let data = bytes.get(offset..offset.checked_add(length)?)?;
        if !drop.contains(&&tag) {
            tables.push((tag, checksum, data));
        }
    }
    tables.sort_by_key(|(tag, ..)| *tag);
    let n = u16::try_from(tables.len()).ok()?;
    let entry_selector = 15u16.saturating_sub(n.max(1).leading_zeros() as u16);
    let search_range = (1u16 << entry_selector).saturating_mul(16);
    let size: usize = tables.iter().map(|(_, _, d)| d.len().div_ceil(4) * 4).sum();
    let mut out = Vec::with_capacity(size.saturating_add(12 + tables.len() * 16));
    out.extend_from_slice(version);
    out.extend_from_slice(&n.to_be_bytes());
    out.extend_from_slice(&search_range.to_be_bytes());
    out.extend_from_slice(&entry_selector.to_be_bytes());
    out.extend_from_slice(&n.saturating_mul(16).saturating_sub(search_range).to_be_bytes());
    let mut offset = 12 + tables.len() * 16;
    for (tag, checksum, data) in &tables {
        out.extend_from_slice(tag);
        out.extend_from_slice(&checksum.to_be_bytes());
        out.extend_from_slice(&u32::try_from(offset).ok()?.to_be_bytes());
        out.extend_from_slice(&u32::try_from(data.len()).ok()?.to_be_bytes());
        offset = offset.checked_add(data.len().div_ceil(4) * 4)?;
    }
    for (_, _, data) in &tables {
        out.extend_from_slice(data);
        out.resize(out.len().div_ceil(4) * 4, 0);
    }
    Some(out)
}

fn be16(bytes: &[u8], at: usize) -> Option<u16> {
    bytes.get(at..at.checked_add(2)?).and_then(|b| b.try_into().ok()).map(u16::from_be_bytes)
}

fn be32(bytes: &[u8], at: usize) -> Option<u32> {
    bytes.get(at..at.checked_add(4)?).and_then(|b| b.try_into().ok()).map(u32::from_be_bytes)
}

/// The faces in one file, from its table directories and `name` tables only.
fn faces_in(path: &Path) -> Option<Vec<SystemFace>> {
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    if len > MAX_FILE {
        return None;
    }
    let mut read_at = |offset: u64, size: usize| -> Option<Vec<u8>> {
        if offset.checked_add(u64::try_from(size).ok()?)? > len {
            return None;
        }
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut buf = vec![0; size];
        file.read_exact(&mut buf).ok()?;
        Some(buf)
    };
    let head = read_at(0, 12)?;
    let starts: Vec<u64> = if head.get(0..4)? == b"ttcf" {
        let count = be32(&head, 8)?.min(MAX_COLLECTION);
        let offsets = read_at(12, usize::try_from(count).ok()?.checked_mul(4)?)?;
        (0..usize::try_from(count).ok()?).filter_map(|i| be32(&offsets, i * 4).map(u64::from)).collect()
    } else {
        vec![0]
    };
    let mut faces = Vec::new();
    for (index, start) in starts.into_iter().enumerate() {
        let Some(directory) = read_at(start, 12) else { continue };
        let tables = usize::from(be16(&directory, 4).unwrap_or(0)).min(256);
        let Some(records) = read_at(start.saturating_add(12), tables * 16) else { continue };
        let table = |tag: &[u8; 4]| {
            (0..tables).find_map(|i| {
                let r = i * 16;
                (records.get(r..r + 4)? == tag).then(|| Some((u64::from(be32(&records, r + 8)?), be32(&records, r + 12)?)))?
            })
        };
        let Some((offset, length)) = table(b"name") else { continue };
        // OS/2 `fsType` (bytes 8–9): restricted (2), preview and print (4) and bitmap-only (0x200)
        // embedding can't be edited in a PDF; a font without the table has no restriction.
        let fs_type = table(b"OS/2").and_then(|(at, _)| read_at(at, 10)).and_then(|os2| be16(&os2, 8)).unwrap_or(0);
        let Some(table) = usize::try_from(length).ok().filter(|l| *l <= 1 << 20).and_then(|l| read_at(offset, l)) else { continue };
        let Some(names) = Names::read(&table) else { continue };
        let Ok(index) = u32::try_from(index) else { continue };
        faces.push(SystemFace {
            path: path.to_path_buf(),
            index,
            family: names.family.clone(),
            full_name: names.full.unwrap_or_else(|| names.family.clone()),
            postscript: names.postscript.unwrap_or_else(|| names.family.replace(' ', "")),
            embeddable: fs_type & (0x0002 | 0x0004 | 0x0200) == 0,
        });
    }
    Some(faces)
}

struct Names {
    family: String,
    full: Option<String>,
    postscript: Option<String>,
}

impl Names {
    /// The family (ID 1), full (4) and PostScript (6) names, in US English where there is a choice.
    fn read(table: &[u8]) -> Option<Self> {
        let count = usize::from(be16(table, 2)?);
        let strings = usize::from(be16(table, 4)?);
        let mut best: [Option<(u8, String)>; 3] = [None, None, None];
        for i in 0..count.min(1024) {
            let r = 6 + i * 12;
            let (platform, encoding, language, id) = (be16(table, r)?, be16(table, r + 2)?, be16(table, r + 4)?, be16(table, r + 6)?);
            let slot = match id {
                1 => 0,
                4 => 1,
                6 => 2,
                _ => continue,
            };
            let length = usize::from(be16(table, r + 8)?);
            let offset = strings.checked_add(usize::from(be16(table, r + 10)?))?;
            let Some(data) = table.get(offset..offset.saturating_add(length)) else { continue };
            let (rank, text) = match (platform, encoding) {
                (3, 0 | 1 | 10) | (0, _) => {
                    let units: Vec<u16> = data.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
                    (if language == 0x0409 || platform == 0 { 3 } else { 2 }, String::from_utf16_lossy(&units))
                }
                (1, 0) => (1, data.iter().map(|b| char::from(*b)).collect()),
                _ => continue,
            };
            let text = text.trim().to_owned();
            if text.is_empty() || text.len() > 128 {
                continue;
            }
            if let Some(entry) = best.get_mut(slot)
                && entry.as_ref().is_none_or(|(r, _)| *r < rank)
            {
                *entry = Some((rank, text));
            }
        }
        let [family, full, postscript] = best;
        Some(Names { family: family?.1, full: full.map(|f| f.1), postscript: postscript.map(|p| p.1) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_face_is_taken_out_of_a_collection() {
        let inter = include_bytes!("../../../assets/fonts/Inter-Regular.ttf");
        // A collection of Inter twice: header, two offsets, then the font, whose table offsets
        // count from the start of the file.
        let at = 12 + 8;
        let mut ttc = b"ttcf\0\x01\0\0\0\0\0\x02".to_vec();
        ttc.extend_from_slice(&(at as u32).to_be_bytes());
        ttc.extend_from_slice(&(at as u32).to_be_bytes());
        let mut font = inter.to_vec();
        let tables = usize::from(be16(&font, 4).unwrap());
        for i in 0..tables {
            let r = 12 + i * 16 + 8;
            let offset = be32(&font, r).unwrap() + at as u32;
            font.splice(r..r + 4, offset.to_be_bytes());
        }
        ttc.extend_from_slice(&font);
        let face = standalone(&ttc, 1).expect("the second face");
        assert!(crate::embedding::validate(&face).is_ok());
        assert_eq!(crate::embedding::names(&face).unwrap(), crate::embedding::names(inter).unwrap());
        assert!(standalone(&ttc, 2).is_none(), "past the collection");
        assert_eq!(standalone(inter, 0).as_deref(), Some(inter.as_slice()));

        let dir = std::env::temp_dir().join(format!("bailancraft-faces-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("pair.ttc"), &ttc).unwrap();
        std::fs::write(dir.join("notes.txt"), b"not a font").unwrap();
        let faces = scan(std::slice::from_ref(&dir));
        assert_eq!(faces.len(), 2);
        assert_eq!(faces[1].index, 1);
        assert_eq!(faces[1].postscript, "Inter-Regular");
        assert!(load(&faces[1]).is_some());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn broken_font_files_list_no_faces() {
        let dir = std::env::temp_dir().join(format!("bailancraft-broken-faces-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("short.ttf"), b"\0\x01").unwrap();
        std::fs::write(dir.join("many.ttc"), b"ttcf\0\x01\0\0\xff\xff\xff\xff\0\0\0\x10").unwrap();
        let faces = scan(std::slice::from_ref(&dir));
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(faces.is_empty());
        assert!(standalone(b"ttcf\0\x01\0\0\0\0\0\x01\xff\xff\xff\xff", 0).is_none());
    }
}
