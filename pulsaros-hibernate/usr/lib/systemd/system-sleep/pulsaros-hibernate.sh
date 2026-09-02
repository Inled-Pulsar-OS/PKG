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
        # Direct session & display restoration
        if [ -x /usr/lib/pulsaros/resume-session ]; then
            /usr/lib/pulsaros/resume-session
        elif [ -x /usr/lib/pulsaros/sleep-progress ]; then
            /usr/lib/pulsaros/sleep-progress post
        fi
    fi
fi

exit 0
