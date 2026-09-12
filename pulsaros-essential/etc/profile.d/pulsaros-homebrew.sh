#!/bin/bash
# English: Homebrew on PATH for every user (installed by pulsaros-essential).
# Español: Homebrew en el PATH para todos los usuarios (instalado por pulsaros-essential).
if [ -d /home/linuxbrew/.linuxbrew/bin ]; then
    case ":$PATH:" in
        *:/home/linuxbrew/.linuxbrew/bin:*) ;;
        *) PATH="/home/linuxbrew/.linuxbrew/bin:$PATH" ;;
    esac
fi
export PATH
