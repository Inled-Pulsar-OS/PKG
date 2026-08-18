#!/bin/sh
# Fix GSConnect config.js paths: replace /usr/local/share with /usr/share
GSCONFIG="/usr/share/gnome-shell/extensions/gsconnect@andyholmes.github.io/config.js"
[ -f "$GSCONFIG" ] && sed -i "s|'/usr/local/share/|'/usr/share/|g" "$GSCONFIG"
