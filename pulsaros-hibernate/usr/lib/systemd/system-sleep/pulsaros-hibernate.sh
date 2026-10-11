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
        # Mitigation: hold a short 'sleep' inhibitor right after resume.
        # gsd-power re-evaluates its inactivity timer on resume and, because
        # the idle clock is not reset, used to order an immediate S3 that left
        # the screen dead just after restoring the session. The grace period
        # gives the session time to come back and the user time to interact.
        if command -v systemd-run >/dev/null 2>&1; then
            systemd-run --collect --quiet \
                --description="Pulsar OS resume grace period" \
                /usr/bin/systemd-inhibit --what=sleep \
                --who="Pulsar OS" --why="Grace period after resume" \
                --mode=block /bin/sleep 30 >/dev/null 2>&1 || true
        fi

        # Direct session & display restoration
        if [ -x /usr/lib/pulsaros/resume-session ]; then
            /usr/lib/pulsaros/resume-session
        elif [ -x /usr/lib/pulsaros/sleep-progress ]; then
            /usr/lib/pulsaros/sleep-progress post
        fi
    fi
fi

exit 0
