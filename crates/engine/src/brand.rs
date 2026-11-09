//! BailanCraft's identity: one place for the name the user sees, the folders it keeps its
//! settings and data in, and whether it may ask the network for updates.
//!
//! BailanCraft is a fork of PdfCraft (MIT OR Apache-2.0). Crate names, command ids, environment
//! variables and other internal identifiers keep the `pdfcraft` spelling so upstream changes merge
//! cleanly; only what a user can see or what must not collide with an installed PdfCraft changes.

use std::borrow::Cow;

/// The product name shown in titles, menus, dialogs and file metadata.
pub const NAME: &str = "BailanCraft";

/// Who makes BailanCraft: shown in About, the update prompt, Windows' Apps list (the MSI's
/// publisher) and the executables' version information.
pub const DEVELOPER: &str = "LANNAFORGE";

/// LANNAFORGE's line, shown under its name in About.
pub const DEVELOPER_TAGLINE: &str = "Born in Chiang Mai, Inspired by Lanna, Built for the World.";

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

/// BailanCraft's GitHub repository, `owner/repo`, as a literal for `concat!`: where its source,
/// issues (feedback) and releases (the update feed) live. Empty until the repository exists;
/// fill in this one place and rebuild to turn on the feedback link and the update check.
#[macro_export]
macro_rules! bailancraft_repository {
    () => {
        "csitotoro/BailanCraft"
    };
}

/// See [`bailancraft_repository!`].
pub const REPOSITORY: &str = bailancraft_repository!();

/// Where users report problems and send feedback (GitHub Issues of [`REPOSITORY`]).
pub const ISSUES_URL: &str = concat!("https://github.com/", bailancraft_repository!(), "/issues");

/// Whether the app asks BailanCraft's own release feed (GitHub Releases of [`REPOSITORY`]) for
/// updates, and offers the feedback link: only once the repository is set. Never the upstream
/// feed, which offers PdfCraft releases that would replace BailanCraft.
pub const UPDATE_CHECK: bool = !REPOSITORY.is_empty();

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
    fn repository_drives_the_feedback_link_and_update_check() {
        assert_eq!(UPDATE_CHECK, !REPOSITORY.is_empty());
        assert!(!REPOSITORY.starts_with("storytold/"), "never the upstream feed");
        assert_eq!(ISSUES_URL, format!("https://github.com/{REPOSITORY}/issues"));
        if UPDATE_CHECK {
            assert_eq!(REPOSITORY.split('/').filter(|p| !p.is_empty()).count(), 2, "owner/repo: {REPOSITORY}");
        }
    }

    #[test]
    fn folders_differ_from_pdfcraft() {
        assert_ne!(DATA_DIR, "PdfCraft");
        assert_ne!(XDG_DIR, "pdfcraft");
        assert_ne!(PORTABLE_DIR, "PdfCraftData");
    }
}
