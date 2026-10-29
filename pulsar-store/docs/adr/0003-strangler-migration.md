# 0003 Migrate from Python incrementally

Status: Accepted

## Context
The Python store ships today. A big-bang rewrite would leave users without updates for weeks and hide regressions.

## Decision
Port `core` first and expose it through `storectl`, a JSON CLI. The Python UI calls `storectl` in place of `store_core.py` (ticket 9.2). The Rust UI is built next, module by module. The Python tree is deleted only when every module has parity (ticket 10.4). Behavior is compared with golden tests on shared fixtures.

## Consequences
- Users get faster search and installs before the new UI exists.
- Temporary duplication: Python and Rust coexist for a while.

## Rejected
- Big-bang rewrite.
- PyO3 bindings: adds Python/Rust packaging complexity for a temporary bridge.

## Amendment (2026-10-28)
The interim bridge (Python UI calling `storectl`, ticket 9.2) was not built. The core is ported and tested headlessly, and the Rust UI follows in the same release train, so the bridge would have been code that is deleted before it ships. The Python tree is still removed only after every module reaches parity (ticket 10.4).
