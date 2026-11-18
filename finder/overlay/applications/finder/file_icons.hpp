#pragma once

// Finder-style document and folder artwork, drawn with Painter primitives at any size.

#include <mayonnaise/mayonnaise.hpp>

#include <cstdint>
#include <vector>

#include "file_model.hpp"

namespace finder {

/// What the thumbnail loader found out about a file's contents.
struct Preview {
    std::shared_ptr<mayo::ImageData> image;  ///< picture thumbnail
    std::vector<std::uint8_t> lines;         ///< text files: length of the first lines (characters, capped)
    bool failed = false;
};

struct IconStyle {
    const Preview* preview = nullptr;
    mayo::Icon folderGlyph = mayo::Icon::None;  ///< embossed on special folders (Downloads, Music...)
    bool dimmed = false;                         ///< cut items
};

/// Draws the icon for `e` centred in the square `r`.
void drawFileIcon(mayo::Painter& p, const mayo::Rect& r, const FileEntry& e, const IconStyle& style = {});
void drawFolderIcon(mayo::Painter& p, const mayo::Rect& r, mayo::Icon glyph = mayo::Icon::None, float opacity = 1);
/// Glyph embossed on the folder icons of well-known places.
mayo::Icon folderGlyphFor(const std::string& path);
/// Sidebar / path bar symbol for a location.
mayo::Icon placeIcon(const std::string& path);

}  // namespace finder
