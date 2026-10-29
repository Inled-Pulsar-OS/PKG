#!/usr/bin/env bash
# Mark every acceptance criterion of the given tickets as done: scripts/tick-ticket.sh 2.1 2.2
set -euo pipefail
cd "$(dirname "$0")/.."
for id in "$@"; do
    for f in docs/tickets/*/"$id"-*.md; do
        sed -i '/^## Acceptance criteria/,/^## Definition/ s/^- \[ \]/- [x]/' "$f"
    done
done
