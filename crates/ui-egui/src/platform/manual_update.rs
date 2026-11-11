//! BailanCraft: updates for a copy that isn't code-signed.
//!
//! `craft_update` installs an update only when this program and the new one carry the same
//! publisher's Authenticode signature; an unsigned BailanCraft answered "this copy of BailanCraft
//! isn't signed … so it doesn't install updates" and the user had to fetch the installer by hand.
//! Here the same prompt's Update button downloads the release's installer from GitHub (checked
//! against the SHA-256 GitHub publishes for it), starts it, and closes BailanCraft so the installer
//! can replace it. A signed copy keeps `craft_update`'s own self-update.

#![cfg(not(target_arch = "wasm32"))]

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};
use std::time::Duration;

use craft_update::release::{Asset, Release};
use craft_update::updater::{CheckSource, Status};
use craft_update::{Http, StartupPrompt, StartupPromptResponse, Stop};

/// Whether this program carries no valid Authenticode signature, so `craft_update` won't
/// install updates for it. Read once. Always false off Windows, where it doesn't apply.
pub(crate) fn unsigned_copy() -> bool {
    #[cfg(windows)]
    {
        static UNSIGNED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *UNSIGNED.get_or_init(|| std::env::current_exe().map_or(true, |exe| verifysign::verify_file(&exe).is_err()))
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// The version this copy counts as: its own, or `BAILANCRAFT_PRETEND_VERSION` to try an update, as
/// craft-update reads it.
fn running_version() -> String {
    let own = env!("CARGO_PKG_VERSION");
    craft_update::self_update::current_version(&crate::UPDATE_APP, own).map_or_else(|| own.to_string(), |v| v.to_string())
}

/// How this copy was installed, as far as choosing the release's file goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// By the MSI (Program Files): the update is the release's `.msi`, run over it.
    Msi,
    /// The portable zip (`portable.txt` beside the program): the update is the zip, unpacked by
    /// hand, so its download page is opened.
    Portable,
}

pub(crate) fn this_kind() -> Kind {
    let portable = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|dir| dir.join("portable.txt").is_file())).unwrap_or(false);
    if portable { Kind::Portable } else { Kind::Msi }
}

/// The release's file for an installation of `kind`: `bailancraft-<version>-windows-x64.msi` or
/// `…-portable.zip`, as `packaging/windows/bailancraft-package.ps1` names them.
pub(crate) fn installer_asset(release: &Release, kind: Kind) -> Option<&Asset> {
    let suffix = match kind {
        Kind::Msi => "-windows-x64.msi",
        Kind::Portable => "-windows-x64-portable.zip",
    };
    release.assets.iter().find(|a| a.name.to_ascii_lowercase().ends_with(suffix))
}

/// The installer can be downloaded and started here: an MSI with a published checksum. Anything
/// else is fetched in the browser.
pub(crate) fn installs_here(release: &Release, kind: Kind) -> bool {
    kind == Kind::Msi && installer_asset(release, kind).is_some_and(|a| a.sha256.as_deref().is_some_and(|h| h.len() == 64))
}

/// What the app should do after [`ManualUpdate::show`].
pub(crate) enum ManualAction {
    None,
    OpenUrl(String),
    /// The installer is running; close through the normal close path.
    Close,
}

#[derive(Default)]
pub(crate) struct ManualUpdate {
    /// Stands in for the network (tests); without it the check asks GitHub.
    pub(crate) source: Option<CheckSource>,
    check: Option<Receiver<Result<Option<Release>, String>>>,
    requested: bool,
    started: bool,
    pub(crate) status: Option<Status>,
    prompt: Option<StartupPrompt>,
    download: Option<Receiver<Result<PathBuf, String>>>,
    close: bool,
}

impl ManualUpdate {
    /// The check at start, once.
    pub(crate) fn check_at_start(&mut self, ctx: &egui::Context) {
        if !std::mem::replace(&mut self.started, true) {
            self.begin(ctx, false);
        }
    }

    /// Help ▸ Check for updates.
    pub(crate) fn check_now(&mut self, ctx: &egui::Context) {
        self.started = true;
        self.begin(ctx, true);
    }

    fn begin(&mut self, ctx: &egui::Context, requested: bool) {
        if self.download.is_some() {
            return;
        }
        if requested {
            self.requested = true;
            self.status = Some(Status::Checking);
        }
        if self.check.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let (ctx, source) = (ctx.clone(), self.source.clone());
        let spawned = std::thread::Builder::new().name("update-check".into()).spawn(move || {
            let ask = || {
                if let Some(ask) = source {
                    return ask();
                }
                let http = Http::new(&format!("{}/Update", crate::UPDATE_APP.name))?;
                craft_update::check_for_update(&crate::UPDATE_APP, &http, &running_version())
            };
            let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(ask)).unwrap_or_else(|_| Err("the update check failed unexpectedly".into()));
            let _ = tx.send(answer);
            ctx.request_repaint();
        });
        match spawned {
            Ok(_) => self.check = Some(rx),
            Err(e) => self.finish(Err(format!("couldn't start the update check ({e})"))),
        }
    }

    fn finish(&mut self, answer: Result<Option<Release>, String>) {
        let requested = std::mem::take(&mut self.requested);
        match answer {
            Ok(Some(release)) => {
                self.status = None;
                if self.prompt.is_none() {
                    self.offer(release);
                }
            }
            Ok(None) => self.status = requested.then_some(Status::UpToDate),
            Err(e) => {
                log::warn!("update check failed: {e}");
                self.status = requested.then_some(Status::Failed(e));
            }
        }
    }

    /// Opens the prompt for `release`: Update downloads and starts its installer when that can
    /// be done here, otherwise Download opens the file in the browser.
    pub(crate) fn offer(&mut self, release: Release) {
        let mut prompt = StartupPrompt::new(&crate::UPDATE_APP, release, running_version());
        prompt.is_self_updating = installs_here(&prompt.release, this_kind());
        self.prompt = Some(prompt);
    }

    pub(crate) fn prompt_open(&self) -> bool {
        self.prompt.is_some()
    }

    /// Picks up a finished check or download. Call once per frame.
    pub(crate) fn tick(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.check {
            match rx.try_recv() {
                Ok(answer) => {
                    self.check = None;
                    self.finish(answer);
                }
                Err(TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
                Err(TryRecvError::Disconnected) => {
                    self.check = None;
                    self.finish(Err("the update check stopped unexpectedly".into()));
                }
            }
        }
        if let Some(rx) = &self.download {
            let outcome = match rx.try_recv() {
                Ok(outcome) => Some(outcome),
                Err(TryRecvError::Empty) => {
                    ctx.request_repaint_after(Duration::from_millis(100));
                    None
                }
                Err(TryRecvError::Disconnected) => Some(Err("the download stopped unexpectedly".into())),
            };
            if let Some(outcome) = outcome {
                self.download = None;
                let started = outcome.and_then(|path| start_installer(&path));
                match started {
                    Ok(()) => {
                        self.prompt = None;
                        self.close = true;
                    }
                    Err(e) => {
                        log::warn!("update failed: {e}");
                        if let Some(prompt) = &mut self.prompt {
                            prompt.is_installing = false;
                            prompt.status_message = Some(e);
                        }
                    }
                }
            }
        }
    }

    /// Draws the prompt. `can_start`: no unsaved work, so the app may close for the installer.
    pub(crate) fn show(&mut self, ctx: &egui::Context, can_start: bool) -> ManualAction {
        if self.close && can_start {
            self.close = false;
            return ManualAction::Close;
        }
        let Some(prompt) = &mut self.prompt else { return ManualAction::None };
        match prompt.show(ctx) {
            StartupPromptResponse::Pending => ManualAction::None,
            StartupPromptResponse::UpdateAndRestart => {
                if self.download.is_some() {
                    return ManualAction::None;
                }
                if !can_start {
                    prompt.status_message = Some("Save or close your open work first; the update closes the app.".into());
                    return ManualAction::None;
                }
                let Some(asset) = installer_asset(&prompt.release, Kind::Msi).cloned() else { return ManualAction::None };
                prompt.is_installing = true;
                prompt.status_message = None;
                let (tx, rx) = channel();
                let ctx = ctx.clone();
                let spawned = std::thread::Builder::new().name("update-download".into()).spawn(move || {
                    let fetch = || download(&asset);
                    let answer = std::panic::catch_unwind(std::panic::AssertUnwindSafe(fetch)).unwrap_or_else(|_| Err("the download failed unexpectedly".into()));
                    let _ = tx.send(answer);
                    ctx.request_repaint();
                });
                match spawned {
                    Ok(_) => self.download = Some(rx),
                    Err(e) => {
                        prompt.is_installing = false;
                        prompt.status_message = Some(format!("couldn't start the download ({e})"));
                    }
                }
                ManualAction::None
            }
            StartupPromptResponse::Download => {
                let url = installer_asset(&prompt.release, this_kind()).map_or_else(|| prompt.release.html_url.clone(), |a| a.url.clone());
                self.prompt = None;
                ManualAction::OpenUrl(url)
            }
            StartupPromptResponse::Later => {
                if self.download.is_none() {
                    self.prompt = None;
                }
                ManualAction::None
            }
        }
    }
}

/// Where the installer is saved: this user's temporary folder, under the app's name.
fn download_dir() -> PathBuf {
    std::env::temp_dir().join(format!("{}-update", crate::UPDATE_APP.id))
}

/// Downloads `asset` (checked against its published SHA-256) and returns where it is.
fn download(asset: &Asset) -> Result<PathBuf, String> {
    let sha256 = asset.sha256.as_deref().ok_or("the release publishes no checksum for its installer")?;
    let name = Path::new(&asset.name).file_name().ok_or("the installer has no file name")?;
    let dest = download_dir().join(name);
    let http = Http::new(&format!("{}/Update", crate::UPDATE_APP.name))?;
    http.download(&asset.url, &dest, asset.size, sha256, &Stop::default(), &mut |_| {})?;
    Ok(dest)
}

/// Starts the Windows Installer on `msi`, which shows its own window and asks to run as
/// administrator; it replaces the installed copy once BailanCraft has closed.
fn start_installer(msi: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        let system = std::env::var_os("SystemRoot").map(PathBuf::from).ok_or("couldn't find Windows")?;
        std::process::Command::new(system.join("System32").join("msiexec.exe"))
            .arg("/i")
            .arg(msi)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("couldn't start the installer ({e})"))
    }
    #[cfg(not(windows))]
    {
        Err(format!("download {} and install it", msi.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn release(assets: &[(&str, Option<&str>)]) -> Release {
        Release {
            tag: "v0.18.3".into(),
            html_url: "https://github.com/csitotoro/BailanCraft/releases/tag/v0.18.3".into(),
            assets: assets
                .iter()
                .map(|(name, hash)| Asset {
                    name: (*name).into(),
                    size: 10,
                    url: format!("https://github.com/csitotoro/BailanCraft/releases/download/v0.18.3/{name}"),
                    sha256: hash.map(str::to_owned),
                })
                .collect(),
            ..Release::default()
        }
    }

    const HASH: &str = "d9bbea085c17430428cc6ee6b6cbcdf67aee5e3bd9b6ebd02f4f39055057fa25";

    #[test]
    fn the_installer_is_the_release_file_for_this_kind_of_copy() {
        let r = release(&[("bailancraft-0.18.3-windows-x64-portable.zip", Some(HASH)), ("bailancraft-0.18.3-windows-x64.msi", Some(HASH))]);
        assert_eq!(installer_asset(&r, Kind::Msi).map(|a| a.name.as_str()), Some("bailancraft-0.18.3-windows-x64.msi"));
        assert_eq!(installer_asset(&r, Kind::Portable).map(|a| a.name.as_str()), Some("bailancraft-0.18.3-windows-x64-portable.zip"));
        assert!(installs_here(&r, Kind::Msi));
        // The portable zip is unpacked by hand; it is downloaded in the browser.
        assert!(!installs_here(&r, Kind::Portable));
    }

    #[test]
    fn an_installer_without_a_checksum_is_downloaded_in_the_browser() {
        let r = release(&[("bailancraft-0.18.3-windows-x64.msi", None)]);
        assert!(!installs_here(&r, Kind::Msi));
        assert!(download(&r.assets[0]).is_err_and(|e| e.contains("checksum")));
        assert!(!installs_here(&release(&[]), Kind::Msi));
    }

    #[test]
    fn a_found_release_opens_the_prompt_and_nothing_found_reports_up_to_date() {
        let mut m = ManualUpdate::default();
        m.requested = true;
        m.finish(Ok(None));
        assert_eq!(m.status, Some(Status::UpToDate));
        assert!(!m.prompt_open());
        m.finish(Ok(Some(release(&[("bailancraft-0.18.3-windows-x64.msi", Some(HASH))]))));
        assert!(m.prompt_open());
        assert_eq!(m.status, None);
        m.requested = true;
        m.finish(Err("offline".into()));
        assert_eq!(m.status, Some(Status::Failed("offline".into())));
    }
}

