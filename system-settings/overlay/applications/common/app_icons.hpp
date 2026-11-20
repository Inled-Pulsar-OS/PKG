#pragma once

// Real application icons for the Forma apps.
//
// The icon is resolved the way the desktop resolves it: the process/application is matched to an
// installed .desktop entry, and that entry's icon name is looked up in the current icon theme (including
// its inherited themes), so the system theme decides the artwork. Nothing is bundled here.
//
// PNG and SVG icons are both supported through gdk-pixbuf. When GIO or gdk-pixbuf is unavailable at build
// time the functions return nullptr and callers keep their letter-tile fallback.

#include <memory>
#include <string>

namespace mayo {
class ImageData;
}

namespace forma {

/// Icon for a process, matched by its comm name and (optionally) its command line. `size` is the pixel
/// size the bitmap is decoded at; callers can draw it smaller. nullptr when no icon could be found.
std::shared_ptr<mayo::ImageData> appIconForProcess(const std::string& processName,
                                                  const std::string& command = {}, int size = 64);

/// Icon for an application display name or desktop id (used by the Settings lists). nullptr when the
/// application is unknown or its icon is not installed in the active theme.
std::shared_ptr<mayo::ImageData> appIconForName(const std::string& displayName, int size = 64);

/// Icon for an explicit themed icon name or absolute path.
std::shared_ptr<mayo::ImageData> themedIcon(const std::string& iconName, int size = 64);

}  // namespace forma
