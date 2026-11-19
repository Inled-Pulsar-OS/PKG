// Public font API (<mayonnaise/font.hpp>): font families, desktop fonts through fontconfig, fallbacks.
//
// libfontconfig is loaded at run time (like libdbus for the portal), so it is not a build dependency; without
// it systemFontFamily() simply finds nothing and callers fall back to Inter or explicit files.
#include <dlfcn.h>

#include <algorithm>
#include <cstdio>
#include <map>
#include <set>
#include <type_traits>

#include "font.hpp"
#include "mayonnaise/font.hpp"

namespace mayo {

namespace {

struct FcPattern;
struct FcConfig;
struct FcObjectSet;
struct FcFontSet {
    int nfont;
    int sfont;
    FcPattern** fonts;
};
constexpr int kResultMatch = 0;
constexpr int kMatchPattern = 0;
constexpr int kWeightRegular = 80;
constexpr int kSlantItalic = 100;
constexpr int kSpacingMono = 100;

struct Fontconfig {
    FcConfig* (*initLoadConfigAndFonts)();
    FcPattern* (*nameParse)(const unsigned char*);
    int (*configSubstitute)(FcConfig*, FcPattern*, int);
    void (*defaultSubstitute)(FcPattern*);
    FcPattern* (*fontMatch)(FcConfig*, FcPattern*, int*);
    int (*getString)(const FcPattern*, const char*, int, unsigned char**);
    int (*getInteger)(const FcPattern*, const char*, int, int*);
    void (*patternDestroy)(FcPattern*);
    FcPattern* (*patternCreate)();
    int (*addInteger)(FcPattern*, const char*, int);
    FcObjectSet* (*objectSetBuild)(const char*, ...);
    void (*objectSetDestroy)(FcObjectSet*);
    FcFontSet* (*fontList)(FcConfig*, FcPattern*, FcObjectSet*);
    void (*fontSetDestroy)(FcFontSet*);
    FcConfig* config = nullptr;

    static Fontconfig* get() {
        static Fontconfig* instance = []() -> Fontconfig* {
            void* lib = dlopen("libfontconfig.so.1", RTLD_NOW | RTLD_LOCAL);
            if (!lib) return nullptr;
            static Fontconfig f{};
            bool ok = true;
            auto sym = [&](auto& fn, const char* name) {
                fn = reinterpret_cast<std::remove_reference_t<decltype(fn)>>(dlsym(lib, name));
                ok = ok && fn;
            };
            sym(f.initLoadConfigAndFonts, "FcInitLoadConfigAndFonts");
            sym(f.nameParse, "FcNameParse");
            sym(f.configSubstitute, "FcConfigSubstitute");
            sym(f.defaultSubstitute, "FcDefaultSubstitute");
            sym(f.fontMatch, "FcFontMatch");
            sym(f.getString, "FcPatternGetString");
            sym(f.getInteger, "FcPatternGetInteger");
            sym(f.patternDestroy, "FcPatternDestroy");
            sym(f.patternCreate, "FcPatternCreate");
            sym(f.addInteger, "FcPatternAddInteger");
            sym(f.objectSetBuild, "FcObjectSetBuild");
            sym(f.objectSetDestroy, "FcObjectSetDestroy");
            sym(f.fontList, "FcFontList");
            sym(f.fontSetDestroy, "FcFontSetDestroy");
            if (!ok) return nullptr;
            f.config = f.initLoadConfigAndFonts();
            return f.config ? &f : nullptr;
        }();
        return instance;
    }

    struct Match {
        FontFile file;
        std::string family;
        int weight = kWeightRegular;
        int slant = 0;
    };

    std::optional<Match> match(std::string_view pattern, bool bold, bool italic) {
        std::string spec(pattern);
        spec += bold ? ":weight=200" : ":weight=80";
        if (italic) spec += ":slant=100";
        FcPattern* pat = nameParse(reinterpret_cast<const unsigned char*>(spec.c_str()));
        if (!pat) return std::nullopt;
        configSubstitute(config, pat, kMatchPattern);
        defaultSubstitute(pat);
        int result = 0;
        FcPattern* found = fontMatch(config, pat, &result);
        patternDestroy(pat);
        if (!found) return std::nullopt;
        Match m;
        unsigned char* s = nullptr;
        if (getString(found, "file", 0, &s) == kResultMatch && s) m.file.path = reinterpret_cast<const char*>(s);
        if (getString(found, "family", 0, &s) == kResultMatch && s) m.family = reinterpret_cast<const char*>(s);
        getInteger(found, "index", 0, &m.file.index);
        getInteger(found, "weight", 0, &m.weight);
        getInteger(found, "slant", 0, &m.slant);
        patternDestroy(found);
        if (m.file.path.empty()) return std::nullopt;
        return m;
    }
};

}  // namespace

std::optional<FontFile> systemFontForCharacter(char32_t cp, bool monospace) {
    Fontconfig* fc = Fontconfig::get();
    if (!fc) return std::nullopt;
    char pattern[64];
    std::snprintf(pattern, sizeof pattern, "%s:charset=%x", monospace ? "monospace" : "sans-serif", static_cast<unsigned>(cp));
    auto m = fc->match(pattern, false, false);
    return m ? std::optional<FontFile>(m->file) : std::nullopt;
}

std::optional<FontFamily> registerFontFamily(const FontFamilyFiles& files) {
    return render::FontLibrary::instance().registerFamily(files);
}

void addFallbackFontFamily(FontFamily family) { render::FontLibrary::instance().addFallback(family); }

std::optional<FontFamily> systemFontFamily(std::string_view pattern) {
    static std::map<std::string, std::optional<FontFamily>, std::less<>> cache;
    if (auto it = cache.find(pattern); it != cache.end()) return it->second;
    std::optional<FontFamily> result;
    if (Fontconfig* fc = Fontconfig::get()) {
        if (auto regular = fc->match(pattern, false, false)) {
            FontFamilyFiles files;
            files.regular = regular->file;
            auto same = [&](const FontFile& f) { return f.path == regular->file.path && f.index == regular->file.index; };
            // Accept a style only if fontconfig found it in the same family; otherwise it is synthesized.
            if (auto b = fc->match(pattern, true, false); b && b->family == regular->family && b->weight >= 180 && !same(b->file))
                files.bold = b->file;
            if (auto i = fc->match(pattern, false, true); i && i->family == regular->family && i->slant >= kSlantItalic && !same(i->file))
                files.italic = i->file;
            if (auto bi = fc->match(pattern, true, true);
                bi && bi->family == regular->family && bi->weight >= 180 && bi->slant >= kSlantItalic && !same(bi->file))
                files.boldItalic = bi->file;
            result = registerFontFamily(files);
        }
    }
    cache.emplace(std::string(pattern), result);
    return result;
}

std::vector<std::string> systemFontFamilies(bool monospaceOnly) {
    std::set<std::string> names;
    if (Fontconfig* fc = Fontconfig::get()) {
        FcPattern* pat = fc->patternCreate();
        if (monospaceOnly) fc->addInteger(pat, "spacing", kSpacingMono);
        FcObjectSet* os = fc->objectSetBuild("family", static_cast<const char*>(nullptr));
        if (FcFontSet* set = fc->fontList(fc->config, pat, os)) {
            for (int i = 0; i < set->nfont; ++i) {
                unsigned char* s = nullptr;
                if (fc->getString(set->fonts[i], "family", 0, &s) == kResultMatch && s) names.insert(reinterpret_cast<const char*>(s));
            }
            fc->fontSetDestroy(set);
        }
        fc->objectSetDestroy(os);
        fc->patternDestroy(pat);
    }
    return {names.begin(), names.end()};
}

LineMetrics fontMetrics(const Font& font) {
    const render::FontMetrics m = render::FontLibrary::instance().metrics(font);
    return {m.ascender, m.descender, m.lineGap, m.capHeight};
}

float glyphAdvance(char32_t cp, const Font& font) { return render::FontLibrary::instance().advance(cp, font); }

}  // namespace mayo
