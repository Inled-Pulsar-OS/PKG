# Conversation Summary: Pulsar OS Recovery → Tauri v2

## Goal
Port the pulsaros-recovery app (Python GTK4 + Rust GTK4) to Tauri v2
with React frontend and Rust backend.

## Architecture Decisions
- **Frontend**: React 19 + TypeScript + Vite + pnpm
- **Backend**: Rust with module-per-concern architecture
- **Plugins**: `@tauri-apps/plugin-shell` (system commands), `@tauri-apps/plugin-http` (downloads)
- **IPC**: Tauri `invoke()` commands, `State<Mutex<>>` for shared state
- **Packaging**: `.deb` target, identifier `es.inled.pulsaros.recovery`
- **CSS**: Tailwind CSS 4 with `@theme` custom tokens
- **Module pattern**: Screen-based modules (psm-front reference)

## Project Structure

### Rust Backend (`src-tauri/src/`)
- `disk.rs` — Btrfs target detection, squashfs detection
- `users.rs` — preserve/restore user accounts (passwd/shadow/group)
- `fstab.rs` — fstab generation with UUID
- `restore.rs` — 8-step restore orchestrator
- `commands/recovery.rs` — Tauri commands exposed to frontend

### React Frontend (`src/`)

#### Core Module (`modules/core/`)
- `types/index.ts` — BtrfsTarget, Screen, RecoveryMode
- `api/commands.ts` — All invoke() wrappers (getBtrfsTargets, startRestore, launchApp, reboot)
- `hooks/use-recovery.ts` — Screen state machine + navigation logic
- `components/recovery-provider.tsx` — React context wrapping useRecovery

#### UI Module (`modules/ui/`)
- `components/screen.tsx` — Card layout wrapper (Tailwind)
- `utils/cn.ts` — clsx + tailwind-merge utility

#### Screen Modules (each self-contained)
- `utilities/` — UtilityRow component, utility data list, UtilitiesPage
- `target-select/` — DiskCard component, TargetSelectPage
- `progress/` — ProgressBar component, ProgressPage
- `complete/` — CompletePage
- `error/` — ErrorPage

#### Styles (`styles/`)
- `globals.css` — Tailwind @import + @theme custom color tokens

## Tailwind Theme Tokens
```css
@theme {
  --color-bg: #1c1c1e;
  --color-card: #2c2c2e;
  --color-border: rgba(255, 255, 255, 0.08);
  --color-border-strong: rgba(255, 255, 255, 0.12);
  --color-accent: #0071e3;
  --color-accent-hover: #007bf5;
  --color-text-secondary: #98989d;
  --color-row-hover: rgba(255, 255, 255, 0.07);
  --color-row-selected: rgba(255, 255, 255, 0.12);
}
```

## Window Config
- `decorations: false` — no title bar
- `fullscreen: true` — fills screen
- Window permissions added to capabilities

## Path Aliases
- `@/*` → `./src/*` (tsconfig.json + vite.config.ts)

## Build Commands
- `cd pulsaros-recovery/tauri && pnpm install`
- `pnpm tauri dev` — development
- `pnpm tauri build` — production (deb)

## Dependencies
- `tailwindcss` + `@tailwindcss/vite` — CSS framework
- `clsx` + `tailwind-merge` — className utilities
- `@tauri-apps/plugin-shell` + `@tauri-apps/plugin-http` — Tauri plugins

## What Was Skipped
- Live log streaming via Tauri Channels (add when needed)
- Custom titlebar buttons (recovery app doesn't need them)
- Arch PKGBUILD (Debian only for now)
