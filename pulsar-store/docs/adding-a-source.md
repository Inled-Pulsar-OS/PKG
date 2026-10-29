# Add a package source

A source knows how to install, remove, and detect one kind of software. Existing ones: Flatpak, system packages, GNOME extensions, Sayri skills and plugins (`src/core/sources/`).

## Steps

1. Add a file `src/core/sources/<name>.rs` with a struct that implements `PackageSource` (`src/core/ports.rs`). Take what you need (a `Runner` for commands, a `Downloader` for files, directories) as constructor arguments so tests can pass fakes.
2. Report progress through the `ProgressSink` and check the `CancelToken` between steps. Return `StoreError::Cancelled` when it is set.
3. Treat ids and URLs from the catalog as untrusted: use `archive::safe_component` for anything that becomes a path, `archive::extract_zip` for archives, `archive::verify_sha256` for downloads, and the `privilege` helper for root. Never build a shell string.
4. Add the source to `SourceKind` in `src/core/model.rs` and to `Sources` and `Installer::source_for` in `src/core/install.rs`. Wire it in `src/core/store.rs`.
5. Prove it with the contract suite. In the tests of `src/core/contract.rs`, add a test that builds a fresh fixture (your source wired to `FakeRunner` and `FakeDownloader`, plus an item it can install) and calls `contract::run`. The suite checks that an item starts not installed, becomes installed with progress events, installs twice, stops on cancel, uninstalls, and updates.
6. Add source-specific tests next to the code for parsing and edge cases.

## Checklist

- [ ] `contract::run` passes for the source
- [ ] No shell strings; no unchecked path from the catalog
- [ ] Downloads are checksum-verified when the catalog declares one
- [ ] `cargo clippy -- -D warnings` is clean
