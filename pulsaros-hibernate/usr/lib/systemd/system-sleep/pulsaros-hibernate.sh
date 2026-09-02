#!/bin/sh
# ==============================================================================
# Pulsar OS - systemd-sleep hook for hibernate/resume session restoration
# ==============================================================================
# $1 = "pre" (before sleep) or "post" (after resume)
# $2 = "hibernate", "suspend", or "hybrid-sleep"

if [ "$2" = "hibernate" ] || [ "$2" = "hybrid-sleep" ]; then
    if [ -x /usr/lib/pulsaros/sleep-progress ]; then
        /usr/lib/pulsaros/sleep-progress "$1"
    fi
fi

exit 0
