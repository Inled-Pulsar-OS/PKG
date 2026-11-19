// Helpers the GNOME and KDE versions of a panel share (previews, thumbnails, font and theme lists).
#pragma once

#include <mayonnaise/mayonnaise.hpp>

#include <filesystem>
#include <functional>
#include <map>
#include <memory>
#include <string>
#include <vector>

#include "ui.hpp"

namespace shared {

/// A miniature desktop in the given appearance (style choices).
void stylePreview(mayo::Painter& p, const mayo::Rect& r, bool dark, mayo::Color accent);
/// Small preview of an image file; worker thread only.
std::shared_ptr<mayo::ImageData> thumbnail(const std::string& path);
/// Draws `img` covering `r` (aspect fill), rounded.
void cover(mayo::Painter& p, const mayo::Rect& r, const mayo::ImageData& img, float radius);
/// Installed font families (fontconfig).
std::vector<std::string> fontFamilies();
/// Theme directories under every icons/ or themes/ data dir that satisfy `accept`.
std::vector<ui::Option> themeDirs(const char* sub, const std::function<bool(const std::filesystem::path&)>& accept);
/// $XDG_DATA_HOME followed by $XDG_DATA_DIRS.
std::vector<std::string> dataDirs();
std::string fileUri(const std::string& path);
bool isImage(const std::string& path);
/// XKB layout and variant descriptions: "us", "tr+f" -> "Turkish (F)".
const std::map<std::string, std::string>& xkbLayoutNames();
bool hasTouchpad();
/// "250 ms".
std::string ms(float v);

}  // namespace shared
