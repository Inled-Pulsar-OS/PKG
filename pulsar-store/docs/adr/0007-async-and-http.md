# 0007 Async model: glib main context, worker threads, `ureq`

Status: Accepted

## Context
GTK runs its own main loop. Adding `tokio` means a second runtime and bridging between them. The launcher uses neither.

## Decision
- UI work runs on the glib main context. Blocking work (network, package managers, disk) runs on worker threads and reports back through channels or `glib::MainContext::spawn_local`.
- HTTP uses `ureq` (blocking), always off the main thread.
- D-Bus uses `zbus` in blocking mode, as the launcher does.
- `core` exposes plain functions and streams (channels). It does not depend on an async runtime.

## Consequences
- One event loop, smaller dependency tree, same style as the launcher.
- Long-lived concurrent I/O is handled with threads; revisit if a feature needs thousands of concurrent tasks.
