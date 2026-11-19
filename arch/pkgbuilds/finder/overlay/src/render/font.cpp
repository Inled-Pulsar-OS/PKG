#include "font.hpp"

#include <ft2build.h>
#include FT_FREETYPE_H
#include FT_ADVANCES_H
#include FT_MULTIPLE_MASTERS_H
#include FT_OUTLINE_H
#include FT_TRUETYPE_TABLES_H

#include <algorithm>
#include <cmath>
#include <cstdio>
#include <cstdlib>
#include <string>
#include <unordered_map>
#include <unordered_set>

#include "../core/utf8.hpp"
#include "embedded.hpp"

namespace mayo {
/// fontconfig lookup of an installed font that has `cp` (font_api.cpp); nullopt without fontconfig.
std::optional<FontFile> systemFontForCharacter(char32_t cp, bool monospace);
}  // namespace mayo

namespace mayo::render {

namespace {

constexpr FT_ULong tag(char a, char b, char c, char d) {
    return (static_cast<FT_ULong>(a) << 24) | (static_cast<FT_ULong>(b) << 16) | (static_cast<FT_ULong>(c) << 8) |
           static_cast<FT_ULong>(d);
}

int opticalSize(float size) { return static_cast<int>(std::lround(std::clamp(size, 14.0f, 32.0f))); }

struct FaceDeleter {
    void operator()(FT_Face f) const { FT_Done_Face(f); }
};
using FacePtr = std::unique_ptr<std::remove_pointer_t<FT_Face>, FaceDeleter>;

/// Cache key for a style: family, weight, italic and (Inter only) optical size.
std::uint64_t styleKey(const Font& font) {
    const auto opsz = font.family == 0 ? static_cast<std::uint64_t>(opticalSize(font.size)) : 0u;
    return (static_cast<std::uint64_t>(font.family) << 19) | (static_cast<std::uint64_t>(font.italic) << 18) |
           (static_cast<std::uint64_t>(font.weight) << 8) | opsz;
}

}  // namespace

struct FontLibrary::Impl {
    /// One FreeType face per (family, file, weight, optical size); ~70 KB each for a variable font. The pixel
    /// size is switched on demand instead of keeping a face per size.
    struct Face {
        FacePtr face;
        FT_F26Dot6 size = -1;  // current char size in 26.6 pixels; -1 = unset
        bool weightAxis = false;
    };
    struct Family {
        bool embedded = false;  // Inter, from memory
        FontFile files[4];      // regular, bold, italic, bold italic
    };
    /// A face ready for a style, plus what has to be faked because the family lacks it.
    struct Resolved {
        FT_Face face = nullptr;
        FontFamily family = 0;  // the family that supplied the face (differs for fallbacks)
        bool synthBold = false;
        bool synthItalic = false;
    };

    FT_Library library = nullptr;
    std::vector<Family> families{Family{true, {}}};
    std::vector<FontFamily> fallbacks;
    std::unordered_map<std::string, FontFamily> familyByFile;  // fonts registered for single characters
    std::unordered_set<char32_t> unavailable;                  // nothing installed has these
    std::unordered_map<std::uint64_t, Face> faces;
    struct Advance {
        float em = 0;
        bool inter = false;  // Inter glyph: gets Inter's tracking
    };
    std::unordered_map<std::uint64_t, Advance> advances;
    std::unordered_map<std::uint64_t, FontMetrics> metrics;  // per em
    FT_UInt wghtAxis = ~0u;
    FT_UInt opszAxis = ~0u;
    FT_UInt axisCount = 0;
    std::vector<FT_Fixed> defaults;

    Impl() {
        if (FT_Init_FreeType(&library)) {
            std::fprintf(stderr, "mayonnaise: FT_Init_FreeType failed\n");
            std::abort();
        }
        FacePtr probe = openInter(400, 14, false);
        FT_MM_Var* mm = nullptr;
        if (FT_Get_MM_Var(probe.get(), &mm) == 0) {
            axisCount = mm->num_axis;
            for (FT_UInt i = 0; i < mm->num_axis; ++i) {
                defaults.push_back(mm->axis[i].def);
                if (mm->axis[i].tag == tag('w', 'g', 'h', 't')) wghtAxis = i;
                if (mm->axis[i].tag == tag('o', 'p', 's', 'z')) opszAxis = i;
            }
            FT_Done_MM_Var(library, mm);
        }
    }

    ~Impl() {
        faces.clear();
        FT_Done_FreeType(library);
    }

    Impl(const Impl&) = delete;
    Impl& operator=(const Impl&) = delete;

    FacePtr openInter(int weight, int opsz, bool applyAxes) const {
        FT_Face face = nullptr;
        const auto& bytes = embedded::InterVariable_ttf;
        if (FT_New_Memory_Face(library, bytes.data(), static_cast<FT_Long>(bytes.size()), 0, &face)) {
            std::fprintf(stderr, "mayonnaise: failed to open embedded Inter font\n");
            std::abort();
        }
        if (applyAxes && axisCount > 0) {
            std::vector<FT_Fixed> coords = defaults;
            if (wghtAxis < axisCount) coords[wghtAxis] = static_cast<FT_Fixed>(weight) << 16;
            if (opszAxis < axisCount) coords[opszAxis] = static_cast<FT_Fixed>(opsz) << 16;
            FT_Set_Var_Design_Coordinates(face, axisCount, coords.data());
        }
        return FacePtr(face);
    }

    /// Opens a font file; a variable font with a weight axis is set to `weight`.
    Face openFile(const FontFile& file, int weight) const {
        FT_Face face = nullptr;
        if (FT_New_Face(library, file.path.c_str(), file.index, &face) != 0) return {};
        Face out{FacePtr(face)};
        FT_MM_Var* mm = nullptr;
        if ((face->face_flags & FT_FACE_FLAG_MULTIPLE_MASTERS) && FT_Get_MM_Var(face, &mm) == 0) {
            std::vector<FT_Fixed> coords(mm->num_axis);
            FT_Get_Var_Design_Coordinates(face, mm->num_axis, coords.data());
            for (FT_UInt i = 0; i < mm->num_axis; ++i) {
                if (mm->axis[i].tag != tag('w', 'g', 'h', 't')) continue;
                coords[i] = std::clamp(static_cast<FT_Fixed>(weight) << 16, mm->axis[i].minimum, mm->axis[i].maximum);
                out.weightAxis = true;
            }
            FT_Set_Var_Design_Coordinates(face, mm->num_axis, coords.data());
            FT_Done_MM_Var(library, mm);
        }
        return out;
    }

    Face* cachedFace(FontFamily family, int slot, int weight, int opsz) {
        const std::uint64_t key = (static_cast<std::uint64_t>(family) << 32) | (static_cast<std::uint64_t>(slot) << 24) |
                                  (static_cast<std::uint64_t>(weight) << 8) | static_cast<std::uint64_t>(opsz);
        auto it = faces.find(key);
        if (it == faces.end()) {
            Face f = families[family].embedded ? Face{openInter(weight, opsz, true), -1, true}
                                               : openFile(families[family].files[slot], weight);
            it = faces.emplace(key, std::move(f)).first;
        }
        return it->second.face ? &it->second : nullptr;
    }

    /// The face for `font`, set to `px64` (26.6 pixels).
    Resolved resolve(const Font& font, FT_F26Dot6 px64) {
        const FontFamily familyId = font.family < families.size() ? font.family : 0;
        const Family& fam = families[familyId];
        const int weight = static_cast<int>(font.weight);
        const bool bold = weight >= 600;
        Resolved r;
        r.family = familyId;
        Face* f = nullptr;
        if (fam.embedded) {
            f = cachedFace(0, 0, weight, opticalSize(font.size));
            r.synthItalic = font.italic;  // Inter is embedded upright only
        } else {
            // Best available slot: exact, then without bold (synthesized), then without italic.
            const int wanted = (bold ? 1 : 0) | (font.italic ? 2 : 0);
            for (int slot : {wanted, wanted & 2, wanted & 1, 0}) {
                if (fam.files[slot].path.empty()) continue;
                // Static files: one face per slot, the weight is the file's own.
                f = cachedFace(familyId, slot, weight, 0);
                if (!f) continue;
                r.synthBold = bold && !(slot & 1) && !f->weightAxis;
                r.synthItalic = font.italic && !(slot & 2);
                break;
            }
            if (!f) return resolve(font.withFamily(0), px64);
        }
        if (f->size != px64) {
            FT_Set_Char_Size(f->face.get(), 0, px64, 72, 72);
            f->size = px64;
        }
        r.face = f->face.get();
        return r;
    }

    /// Like resolve(), but walks the fallback families when the font has no glyph for `cp`.
    Resolved resolveGlyph(char32_t cp, const Font& font, FT_F26Dot6 px64, FT_UInt& glyph) {
        Resolved r = resolve(font, px64);
        glyph = FT_Get_Char_Index(r.face, cp);
        if (glyph != 0 || cp < 0x20) return r;
        for (FontFamily fb : fallbacks) {
            if (fb == font.family) continue;
            Resolved alt = resolve(font.withFamily(fb), px64);
            if (FT_UInt g = FT_Get_Char_Index(alt.face, cp)) {
                glyph = g;
                return alt;
            }
        }
        if (font.family != 0) {
            Resolved inter = resolve(font.withFamily(0), px64);
            if (FT_UInt g = FT_Get_Char_Index(inter.face, cp)) {
                glyph = g;
                return inter;
            }
        }
        // Ask the system for any installed font with this character (CJK, symbols, Nerd Font icons) and
        // keep it as a fallback for the characters that follow.
        if (!unavailable.count(cp)) {
            const bool mono = font.family != 0 && FT_IS_FIXED_WIDTH(r.face);
            if (auto file = systemFontForCharacter(cp, mono)) {
                const std::string key = file->path + "#" + std::to_string(file->index);
                FontFamily fam = 0;
                if (auto it = familyByFile.find(key); it != familyByFile.end()) {
                    fam = it->second;
                } else if (Face probe = openFile(*file, 400); probe.face && FT_Get_Char_Index(probe.face.get(), cp) != 0 &&
                                                              families.size() < 0xFFFF) {
                    Family f;
                    f.files[0] = *file;
                    families.push_back(std::move(f));
                    fam = static_cast<FontFamily>(families.size() - 1);
                    familyByFile.emplace(key, fam);
                    fallbacks.push_back(fam);
                }
                if (fam != 0) {
                    Resolved alt = resolve(font.withFamily(fam), px64);
                    if (FT_UInt g = FT_Get_Char_Index(alt.face, cp)) {
                        glyph = g;
                        return alt;
                    }
                }
            }
            unavailable.insert(cp);
        }
        return r;  // .notdef of the requested family
    }

    /// One pixel per font unit: advances come back as exact (variation-adjusted) design units.
    static FT_F26Dot6 unitsSize(FT_Face face) { return static_cast<FT_F26Dot6>(face->units_per_EM) * 64; }
};

FontLibrary& FontLibrary::instance() {
    static FontLibrary library;
    return library;
}

FontLibrary::FontLibrary() : m_impl(std::make_unique<Impl>()) {}
FontLibrary::~FontLibrary() = default;

float FontLibrary::tracking(float size) {
    // https://rsms.me/inter/dynmetrics/
    constexpr float a = -0.0223f;
    constexpr float b = 0.185f;
    constexpr float c = -0.1745f;
    return a + b * std::exp(c * size);
}

std::optional<FontFamily> FontLibrary::registerFamily(const FontFamilyFiles& files) {
    if (m_impl->families.size() >= 0xFFFF) return std::nullopt;
    Impl::Family fam;
    fam.files[0] = files.regular;
    fam.files[1] = files.bold;
    fam.files[2] = files.italic;
    fam.files[3] = files.boldItalic;
    // The regular face must open, or the family is useless.
    if (!m_impl->openFile(fam.files[0], 400).face) return std::nullopt;
    m_impl->families.push_back(std::move(fam));
    return static_cast<FontFamily>(m_impl->families.size() - 1);
}

void FontLibrary::addFallback(FontFamily family) {
    if (family < m_impl->families.size() &&
        std::find(m_impl->fallbacks.begin(), m_impl->fallbacks.end(), family) == m_impl->fallbacks.end())
        m_impl->fallbacks.push_back(family);
    m_impl->advances.clear();  // characters that used to be missing may now come from the fallback
}

float FontLibrary::advance(char32_t cp, const Font& font) {
    const std::uint64_t key = (styleKey(font) << 21) | static_cast<std::uint64_t>(cp & 0x1FFFFF);
    auto it = m_impl->advances.find(key);
    Impl::Advance a;
    if (it != m_impl->advances.end()) {
        a = it->second;
    } else {
        FT_UInt gi = 0;
        // Find the family that has the glyph, then measure at one pixel per font unit of that face.
        const Impl::Resolved found = m_impl->resolveGlyph(cp, font, 64 * 64, gi);
        Impl::Resolved r = m_impl->resolve(font.withFamily(found.family), Impl::unitsSize(found.face));
        FT_Fixed adv = 0;
        FT_Get_Advance(r.face, gi, FT_LOAD_NO_HINTING, &adv);
        a.em = static_cast<float>(adv) / 65536.0f / static_cast<float>(r.face->units_per_EM);
        a.inter = r.family == 0;
        m_impl->advances.emplace(key, a);
    }
    return (a.em + (a.inter ? tracking(font.size) : 0.0f)) * font.size;
}

FontMetrics FontLibrary::metrics(const Font& font) {
    auto key = styleKey(font);
    auto it = m_impl->metrics.find(key);
    FontMetrics em;
    if (it != m_impl->metrics.end()) {
        em = it->second;
    } else {
        FT_Face face = m_impl->resolve(font, 64).face;
        auto upem = static_cast<float>(face->units_per_EM);
        em.ascender = static_cast<float>(face->ascender) / upem;
        em.descender = -static_cast<float>(face->descender) / upem;
        em.lineGap = std::max(0.0f, static_cast<float>(face->height) / upem - em.ascender - em.descender);
        auto* os2 = static_cast<TT_OS2*>(FT_Get_Sfnt_Table(face, FT_SFNT_OS2));
        em.capHeight = os2 && os2->version >= 2 ? static_cast<float>(os2->sCapHeight) / upem : 0.727f;
        em.xHeight = os2 && os2->version >= 2 ? static_cast<float>(os2->sxHeight) / upem : 0.546f;
        m_impl->metrics.emplace(key, em);
    }
    return {em.ascender * font.size, em.descender * font.size, em.capHeight * font.size, em.xHeight * font.size,
            em.lineGap * font.size};
}

bool FontLibrary::rasterize(char32_t cp, const Font& font, float pixelSize, int subpixel, GlyphBitmap& out) {
    FT_UInt gi = 0;
    const auto px64 = static_cast<FT_F26Dot6>(std::lround(pixelSize * 64.0f));
    Impl::Resolved r = m_impl->resolveGlyph(cp, font, px64, gi);
    FT_Face face = r.face;
    // Unhinted outlines keep glyph shapes and spacing faithful to the design, like macOS.
    if (FT_Load_Glyph(face, gi, FT_LOAD_NO_HINTING | FT_LOAD_NO_BITMAP)) return false;
    if (face->glyph->format != FT_GLYPH_FORMAT_OUTLINE) return false;
    FT_Outline& outline = face->glyph->outline;
    if (r.synthItalic) {
        // ~12 degree slant, like the oblique styles of most UI fonts.
        FT_Matrix shear{0x10000, 0x0366A, 0, 0x10000};
        FT_Outline_Transform(&outline, &shear);
    }
    if (r.synthBold) {
        const FT_Pos strength = std::max<FT_Pos>(32, px64 / 22);
        FT_Outline_EmboldenXY(&outline, strength, 0);
        FT_Outline_Translate(&outline, -strength / 2, 0);  // keep it centred in its advance
    }
    FT_Outline_Translate(&outline, subpixel * 16, 0);
    if (FT_Render_Glyph(face->glyph, FT_RENDER_MODE_NORMAL)) return false;
    const FT_Bitmap& bm = face->glyph->bitmap;
    out.width = static_cast<int>(bm.width);
    out.height = static_cast<int>(bm.rows);
    out.left = face->glyph->bitmap_left;
    out.top = face->glyph->bitmap_top;
    out.pixels.resize(static_cast<size_t>(out.width) * static_cast<size_t>(out.height));
    for (int y = 0; y < out.height; ++y) {
        const unsigned char* row = bm.buffer + static_cast<std::ptrdiff_t>(y) * bm.pitch;
        std::copy(row, row + out.width, out.pixels.begin() + static_cast<std::ptrdiff_t>(y * out.width));
    }
    return true;
}

// --- layout helpers --------------------------------------------------------------------------

float measureText(std::string_view text, const Font& font) {
    auto& lib = FontLibrary::instance();
    float w = 0;
    size_t i = 0;
    while (i < text.size()) w += lib.advance(utf8::next(text, i), font);
    return w;
}

std::vector<TextLine> wrapText(std::string_view text, const Font& font, float maxWidth) {
    auto& lib = FontLibrary::instance();
    std::vector<TextLine> lines;
    const bool wrap = maxWidth > 0 && std::isfinite(maxWidth);
    size_t lineStart = 0;
    float lineWidth = 0;
    size_t lastBreak = std::string_view::npos;  // byte index just after a space
    float widthAtBreak = 0;                     // width up to (excluding) that space
    size_t i = 0;
    while (i < text.size()) {
        size_t cpStart = i;
        char32_t cp = utf8::next(text, i);
        if (cp == '\n') {
            lines.push_back({lineStart, cpStart, lineWidth});
            lineStart = i;
            lineWidth = 0;
            lastBreak = std::string_view::npos;
            continue;
        }
        float adv = lib.advance(cp, font);
        if (cp == ' ') {
            lastBreak = i;
            widthAtBreak = lineWidth;
            lineWidth += adv;
            continue;
        }
        if (wrap && lineWidth + adv > maxWidth && cpStart > lineStart) {
            if (lastBreak != std::string_view::npos) {
                lines.push_back({lineStart, lastBreak - 1, widthAtBreak});
                lineStart = lastBreak;
                lineWidth = measureText(text.substr(lineStart, cpStart - lineStart), font);
            } else {
                // A single word longer than the line: break it mid-word.
                lines.push_back({lineStart, cpStart, lineWidth});
                lineStart = cpStart;
                lineWidth = 0;
            }
            lastBreak = std::string_view::npos;
        }
        lineWidth += adv;
    }
    lines.push_back({lineStart, text.size(), lineWidth});
    return lines;
}

void caretPositions(std::string_view text, const Font& font, std::vector<size_t>& boundaries,
                    std::vector<float>& positions) {
    auto& lib = FontLibrary::instance();
    boundaries.clear();
    positions.clear();
    float x = 0;
    size_t i = 0;
    boundaries.push_back(0);
    positions.push_back(0);
    while (i < text.size()) {
        x += lib.advance(utf8::next(text, i), font);
        boundaries.push_back(i);
        positions.push_back(x);
    }
}

}  // namespace mayo::render
