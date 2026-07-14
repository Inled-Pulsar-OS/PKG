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
    aplay -D "plughw:$target_card,0" -q "$SOUND_FILE"
else
    echo "🔊 No sound cards detected. Falling back to default."
    aplay -q "$SOUND_FILE"
fi
