# BailanCraft: instructions for agents

BailanCraft is a fork of PdfCraft (storytold/pdfcraft, MIT OR Apache-2.0), a Rust-native PDF app
(egui UI, hayro rendering). The fork's goals, in order: correct Thai everywhere (consonants, vowels
and tone marks placed right, copy/search/extract giving the right text, Thai line breaking, Thai
OCR, Thai in forms and Fill & Sign); its own identity (name, palm-leaf ("ใบลาน") look); a Windows
installer for distribution inside the owner's company and beyond. The owner writes in Thai; answer
in Thai.

The upstream agent instructions are kept in `docs/UPSTREAM-CLAUDE.md`. Its engineering rules still
apply here (below); its process rules (PRs into storytold/pdfcraft `main`, `gh` searches of the
upstream tracker, `plan/` files, parity bookkeeping, CI policy) do not.

## Branches and upstream
- Work happens on `bailancraft`. `upstream` is storytold/pdfcraft. Bring upstream in with
  `git fetch upstream && git merge upstream/main` and read every conflicting hunk.
- Keep fork changes small and in few places so merges stay easy:
  - Identity lives in `crates/engine/src/brand.rs` (name, data folders, `UPDATE_CHECK`). Use its
    constants; don't hard-code "BailanCraft" or folder names elsewhere.
  - User-visible "PdfCraft" in translated or English UI text is rewritten at the translation layer
    (`branded` in `crates/ui-egui/src/i18n.rs`). Don't edit catalog keys or `tl!` strings to rename.
  - Keep crate names, command ids, `PDFCRAFT_*` env vars and other internal `pdfcraft` identifiers.
  - Mark fork-only edits with a short `BailanCraft:` comment.
- Never ship ArtCraft names or logos (`docs/brand/LICENSE-brand.txt`). Credit PdfCraft in About.

## Build and run (Windows)
- Build inputs beside the repo: `../craft-fonts` (Thai, Arabic, Hebrew, CJK faces; commit pinned in
  `.github/workflows/ci.yml`) and `../craft-i18n` (UI translations incl. `th`; commit in
  `craft-i18n.rev`). Without `CRAFT_FONTS_DIR`, Thai Add Text is refused.
- `CRAFT_FONTS_DIR=../craft-fonts CRAFT_I18N_DIR=../craft-i18n cargo build --release -p pdfcraft -j 8`
  (about 20 min cold). Same for `-p pdfcraft-cli`. Scope builds and tests to touched crates
  (`cargo test -p <crate> -j 4`); a full `--workspace` build is slow on this machine.
- Look at UI changes: `target/release/pdfcraft.exe --control ctl.txt file.pdf`, then
  `pdfcraft-cli ui --control ctl.txt screenshot --out x.png` (also `set key=language value=th`).
- Engine checks without the UI: `pdfcraft-cli run --script steps.json --root DIR`
  (steps are `{"tool": ..., "args": {...}, "out": "x.png"}`; `pdfcraft-cli tools` lists tools).
- Thai test script and outputs live outside the repo in `../BailanCraft-test/`.

## Engineering rules kept from upstream
- Never panic on input: no `unwrap`/`expect`/indexing on input-derived data in non-test code,
  checked arithmetic, char-boundary slicing, capped allocations, handle lock poisoning, no `unsafe`.
- Every crash fix gets a synthetic regression test; prove a new test fails without the fix.
- Assets: openly licensed or original only, each with an `ATTRIBUTION.toml` entry (`AGENTS.md` §1).
  Fonts come from craft-fonts, not this repo. No GPL/AGPL code.
- Layering: nothing below the UI layer depends on egui/winit/eframe.
- Rust only in the product and the build.

## Known Thai gaps (the fork's backlog)
- Line breaking splits Thai words (no dictionary segmentation; `crates/edit/src/linebreak.rs`).
- OCR has no Thai (ocrs models are Latin-only; Tesseract allow-list lacks `tha`, `crates/ocr`).
- Forms and Fill & Sign don't write Thai; `doc_create` from text is WinAnsi-only.
- Some UI strings have no Thai translation yet (e.g. the update dialog).

## Known test notes (this machine)
- `pdfcraft` bin: three `single_instance` socket tests fail here (the AF_UNIX socket file is not
  left behind, hand-off never connects), inside and outside the sandbox. They don't touch fork
  code. In the real app a second launch opens its own window instead of a tab. Not yet compared
  with upstream on another machine.
- `pdfcraft-ui-egui --test it` once died with STATUS_ACCESS_VIOLATION mid-run under load (native
  code); a rerun passed. Rerun before suspecting a change.
- Six upstream update-flow tests are `#[ignore]`d because `brand::UPDATE_CHECK` is off;
  `bailancraft_never_asks_the_upstream_release_feed` covers the fork's behaviour.
