#!/bin/sh
# ==============================================================================
# Pulsar OS - systemd-sleep hook for hibernate pre/post
# In systemd v253+: $1=pre|post, $2=sleep_operation (e.g. hibernate)
# In older systemd: $1=pre|post, $2=sleep_type
# ==============================================================================

if [ "$1" = "pre" ]; then
    if [ "$2" = "hibernate" ] || [ "$2" = "hybrid-sleep" ] || [ "$2" = "suspend-then-hibernate" ]; then
        [ -x /usr/lib/pulsaros/sleep-progress ] && /usr/lib/pulsaros/sleep-progress pre
    fi
elif [ "$1" = "post" ]; then
    if [ "$2" = "hibernate" ] || [ "$2" = "hybrid-sleep" ] || [ "$2" = "suspend-then-hibernate" ]; then
        [ -x /usr/lib/pulsaros/sleep-progress ] && /usr/lib/pulsaros/sleep-progress post
        # Start the graphical session restoration service in the background.
        # systemctl start will block until it finishes, which is fine here since
        # this hook runs asynchronously relative to the display manager.
        systemctl start pulsaros-resume-session.service 2>/dev/null || true
    fi
fi

exit 0
