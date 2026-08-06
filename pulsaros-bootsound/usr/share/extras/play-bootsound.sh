#!/bin/bash
# ==============================================================================
# play-bootsound.sh - Smart Bootsound Player for Pulsar OS
# ==============================================================================

SOUND_FILE="/usr/share/extras/boot-sound.wav"

if [ ! -f "$SOUND_FILE" ]; then
    echo "❌ Boot sound file not found." >&2
    exit 1
fi

# 1. Try to find card by looking for Analog/Speaker/Headphone
target_card=$(aplay -l | grep -i -E 'analog|speaker|headphone' | awk '{print $2}' | sed 's/://' | head -n 1)

# 2. Fallback: exclude HDMI, Nvidia, DisplayPort, SPDIF
if [ -z "$target_card" ]; then
    target_card=$(aplay -l | grep -i 'card' | grep -v -E -i 'hdmi|nvidia|displayport|s/pdif' | awk '{print $2}' | sed 's/://' | head -n 1)
fi

# 3. Fallback: first card
if [ -z "$target_card" ]; then
    target_card=$(aplay -l | grep -i 'card' | awk '{print $2}' | sed 's/://' | head -n 1)
fi

if [ -n "$target_card" ]; then
    echo "🔊 Selected target card: $target_card (plughw:$target_card,0)"
    # Unmute and set master/outputs volume to 100%
    amixer -c "$target_card" set Master unmute 100% >/dev/null 2>&1
    amixer -c "$target_card" set PCM unmute 100% >/dev/null 2>&1
    amixer -c "$target_card" set Headphone unmute 100% >/dev/null 2>&1
    amixer -c "$target_card" set Speaker unmute 100% >/dev/null 2>&1
    aplay -D "plughw:$target_card,0" -q "$SOUND_FILE"
else
    echo "🔊 No sound cards detected. Falling back to default."
    amixer set Master unmute 100% >/dev/null 2>&1
    amixer set PCM unmute 100% >/dev/null 2>&1
    amixer set Headphone unmute 100% >/dev/null 2>&1
    amixer set Speaker unmute 100% >/dev/null 2>&1
    aplay -q "$SOUND_FILE"
fi
