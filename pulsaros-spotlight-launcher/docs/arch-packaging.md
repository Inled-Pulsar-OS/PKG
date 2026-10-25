# Arch packaging tickets

Source: review of `arch/pkgbuilds/pulsaros-spotlight-launcher/PKGBUILD`. Status after this change
is in the last column. Verified with `makepkg -f --nodeps` in a scratch `BUILDDIR`/`PKGDEST`
(build, `check()` with 151 tests, package contents, `.PKGINFO`). Not verified: install on a real
GNOME session, `namcap` (not installed), clean-chroot build.

| ID | Problem | Fix | Status |
|---|---|---|---|
| P1 | `arch=(any)` with a compiled binary | `arch=(x86_64 aarch64)` | Done |
| P2 | `build()` rebuilt only when `src/main.rs` was newer, so edits elsewhere shipped a stale binary | Always `cargo build --release --locked`; cargo skips unchanged work | Done |
| P3 | 5 MB stale binary tracked at `arch/pkgbuilds/pulsaros-spotlight-launcher/pulsaros-spotlight` | Removed from git, ignore pattern added | Done |
| P4 | Missing runtime deps (`sqlite`, `json-glib` linked by the binary; `curl`, `glib2` used at runtime) | Added to `depends`; optional tools added to `optdepends` (poppler, ffmpeg, libnotify, xdg-utils, networkmanager, wireplumber, playerctl) | Done |
| P5 | `ydotool` was a hard dependency (Debian only recommends it) | Moved to `optdepends` | Done |
| P6 | `gnome-shell` missing (the extension needs it; Debian depends on it) | Added to `depends` | Done |
| P7 | `\|\| true` hid install failures for icons/CSS | Removed; any missing file now fails the package | Done |
| P8 | `license=(GPL3)` deprecated; `Cargo.toml` had no license | `GPL-3.0-or-later` in both | Done |
| P9 | Debian provides/conflicts `pulsaros-spotlight`, Arch did not | Added to `provides` and `conflicts` | Done |
| P10 | `options=(!strip)` kept a 5 MB unstripped binary; makepkg also produced an empty `-debug` package | Dropped `!strip`; added `options=(!debug)` | Done |
| P11 | Build wrote `target/` into the source tree | `CARGO_TARGET_DIR="$srcdir/target"` | Done |
| P12 | No `check()` | Added `cargo test`, skipping `thumb_generates_scaled_from_real_image` (needs a Pulsar OS wallpaper) | Done |
| P13 | `pkgrel` | Bumped to 5 | Done |
| P14 | Debian `control` lacked the same new deps | Synced `Depends`/`Recommends` | Done |

## Open

- **O1.** Sources come from `$startdir/../../..`, so the PKGBUILD only builds inside this monorepo, and a clean chroot (`extra-x86_64-build`) cannot see them. Fix: `source=()` from a git tag or `git+file://` once releases are tagged.
- **O2.** `replaces=(spotlight-gtk spotlight-python)` makes pacman swap packages on upgrade. Keep only if those old packages are still in the repo.
- **O3.** Run `namcap` and an install test on a GNOME machine (extension loads, desktop entry and icons show).
- **O4.** `DEBIAN/changelog` has an uncommitted entry that was not made by this change; review it separately.
