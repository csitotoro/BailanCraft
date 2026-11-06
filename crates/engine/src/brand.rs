//! BailanCraft's identity: one place for the name the user sees, the folders it keeps its
//! settings and data in, and whether it may ask the network for updates.
//!
//! BailanCraft is a fork of PdfCraft (MIT OR Apache-2.0). Crate names, command ids, environment
//! variables and other internal identifiers keep the `pdfcraft` spelling so upstream changes merge
//! cleanly; only what a user can see or what must not collide with an installed PdfCraft changes.

use std::borrow::Cow;

/// The product name shown in titles, menus, dialogs and file metadata.
pub const NAME: &str = "BailanCraft";

/// The upstream product this fork is built from, credited in About.
pub const UPSTREAM_NAME: &str = "PdfCraft";

/// Where the upstream source lives (credited in About and the Help menu).
pub const UPSTREAM_SOURCE: &str = "https://github.com/storytold/pdfcraft";

/// Per-user folder name on Windows (`%LOCALAPPDATA%`, `%APPDATA%`) and macOS: different from
/// PdfCraft's, so the two apps never share settings, recovery files or the single-instance socket.
pub const DATA_DIR: &str = "BailanCraft";

/// Lower-case folder name for XDG desktops (`~/.config/…`, `~/.local/share/…`, `$XDG_RUNTIME_DIR/…`).
pub const XDG_DIR: &str = "bailancraft";

/// Folder beside the executable that switches on portable mode.
pub const PORTABLE_DIR: &str = "BailanCraftData";

/// Whether the app may ask the upstream release feed for updates. Off: that feed offers PdfCraft
/// releases, which would replace BailanCraft. Turn on once BailanCraft has its own feed.
pub const UPDATE_CHECK: bool = false;

/// `text` with the upstream product name replaced by [`NAME`]. Borrowed when there is nothing
/// to replace, which is nearly always.
pub fn rebrand(text: &str) -> Cow<'_, str> {
    if text.contains(UPSTREAM_NAME) { Cow::Owned(text.replace(UPSTREAM_NAME, NAME)) } else { Cow::Borrowed(text) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebrand_replaces_only_the_product_name() {
        assert_eq!(rebrand("Welcome to PdfCraft"), "Welcome to BailanCraft");
        assert_eq!(rebrand("ยินดีต้อนรับสู่ PdfCraft"), "ยินดีต้อนรับสู่ BailanCraft");
        assert!(matches!(rebrand("Open"), Cow::Borrowed("Open")));
        assert_eq!(rebrand(""), "");
    }

    #[test]
    fn folders_differ_from_pdfcraft() {
        assert_ne!(DATA_DIR, "PdfCraft");
        assert_ne!(XDG_DIR, "pdfcraft");
        assert_ne!(PORTABLE_DIR, "PdfCraftData");
    }
}
