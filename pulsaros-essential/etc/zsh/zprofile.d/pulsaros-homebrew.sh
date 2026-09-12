#!/bin/bash
# English: Homebrew on PATH for zsh login shells (installed by pulsaros-essential).
# Español: Homebrew en el PATH para shells de login zsh (instalado por pulsaros-essential).
if [ -d /home/linuxbrew/.linuxbrew/bin ]; then
    case ":$PATH:" in
        *:/home/linuxbrew/.linuxbrew/bin:*) ;;
        *) PATH="/home/linuxbrew/.linuxbrew/bin:$PATH" ;;
    esac
fi
export PATH
