#!/bin/bash
# ==============================================================================
# play-bootsound.sh - Smart Bootsound Player for Pulsar OS
# ==============================================================================

SOUND_FILE="/usr/share/extras/boot-sound.wav"

if [ ! -f "$SOUND_FILE" ]; then
    echo "❌ Boot sound file not found." >&2
    exit 1
fi

# Find all card numbers
cards=$(aplay -l | grep -i 'card' | awk -F' ' '{print $2}' | sed 's/://' | sort -u)

target_card=""
for card in $cards; do
    # Check if this card has a device 0 (HDMI usually has device 3/7/8/9, analog usually has device 0)
    has_dev0=$(aplay -l | grep -i "card $card:" | grep -i "device 0" || true)
    if [ -z "$has_dev0" ]; then
        continue
    fi
    
    card_info=$(aplay -l | grep -i "card $card:")
    # Exclude HDMI/NVidia/SPDIF/Digital
    if echo "$card_info" | grep -E -i -q 'hdmi|nvidia|s/pdif|digital'; then
        continue
    fi
    
    target_card="$card"
    break
done

if [ -z "$target_card" ]; then
    # Fallback to the first card with device 0
    for card in $cards; do
        has_dev0=$(aplay -l | grep -i "card $card:" | grep -i "device 0" || true)
        if [ -n "$has_dev0" ]; then
            target_card="$card"
            break
        fi
    done
fi

if [ -z "$target_card" ]; then
    # Fallback to first card in general
    target_card=$(echo "$cards" | head -n 1)
fi

if [ -n "$target_card" ]; then
    echo "🔊 Selected target card: $target_card (plughw:$target_card,0)"
    aplay -D "plughw:$target_card,0" -q "$SOUND_FILE"
else
    echo "🔊 No sound cards detected. Falling back to default."
    aplay -q "$SOUND_FILE"
fi
