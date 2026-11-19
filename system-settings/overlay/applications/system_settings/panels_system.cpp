// Privacy & Security, Wellbeing, Region & Language (AccountsService, localed), Date & Time (timedated),
// Users (AccountsService) and About (hostnamed, /proc, switcheroo-control, os-release).
#include <sys/statvfs.h>
#include <sys/utsname.h>
#include <unistd.h>

#include <algorithm>
#include <clocale>
#include <cstdio>
#include <ctime>
#include <filesystem>
#include <fstream>
#include <map>
#include <set>
#include <sstream>

#include "glib_support.hpp"
#include "json.hpp"
#include "kconfig.hpp"
#include "panels.hpp"
#include "ui.hpp"

using namespace mayo;
namespace fs = std::filesystem;

// Privacy & Security --------------------------------------------------------------------------------------

Element privacyPanel(Context& ctx) {
    if (desk::kde()) return kde::privacyPanel(ctx);
    constexpr const char* kPrivacy = "org.gnome.desktop.privacy";
    constexpr const char* kScreensaver = "org.gnome.desktop.screensaver";

    auto clearHistory = Button("Clear History…");
    clearHistory.onClick([anchor = ui::Anchor(clearHistory)] {
        ui::confirm(anchor, "Clear file history?", "The list of recently used files is emptied for every app.", "Clear History", [] {
            if (gx::readOnly()) return;
            // An empty bookmark file is what GtkRecentManager writes after purging.
            const fs::path path = fs::path(g_get_user_data_dir()) / "recently-used.xbel";
            std::ofstream out(path, std::ios::trunc);
            out << "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n"
                   "<xbel version=\"1.0\" xmlns:bookmark=\"http://www.freedesktop.org/standards/desktop-bookmarks\" "
                   "xmlns:mime=\"http://www.freedesktop.org/standards/shared-mime-info\">\n</xbel>\n";
        });
    });
    auto emptyTrash = Button("Empty Trash…");
    emptyTrash.onClick([anchor = ui::Anchor(emptyTrash), appAnchor = ctx.anchor] {
        ui::confirm(anchor, "Empty the Trash?", "All items in the Trash are permanently deleted.", "Empty Trash", [appAnchor] {
            gx::run({"gio", "trash", "--empty"}, [appAnchor](const gx::ProcessResult& r) {
                if (!r.ok()) ui::message(appAnchor, "Couldn't empty the Trash", r.err);
            });
        });
    });

    std::vector<Element> lock{
        ui::toggleRow("Automatic Screen Lock", gx::Key(kScreensaver, "lock-enabled"), "Locks the screen when it turns off"),
        ui::pickerRow("Lock Delay", gx::Key(kScreensaver, "lock-delay"),
                      {{"When the Screen Turns Off", "0"}, {"30 seconds", "30"}, {"1 minute", "60"}, {"2 minutes", "120"},
                       {"3 minutes", "180"}, {"5 minutes", "300"}, {"30 minutes", "1800"}, {"1 hour", "3600"}}),
        ui::pickerRow("Blank Screen Delay", gx::Key("org.gnome.desktop.session", "idle-delay"),
                      {{"1 minute", "60"}, {"2 minutes", "120"}, {"3 minutes", "180"}, {"5 minutes", "300"}, {"10 minutes", "600"},
                       {"15 minutes", "900"}, {"Never", "0"}}),
        ui::toggleRow("Lock Screen Notifications", gx::Key("org.gnome.desktop.notifications", "show-in-lock-screen")),
    };
    gx::Key usb(kPrivacy, "usb-protection");
    if (usb && gx::hasName(gx::systemBus(), "org.usbguard1"))
        lock.push_back(ui::toggleRow("USB Protection", usb, "Blocks new USB devices while the screen is locked"));

    return ui::page({
        ui::section("Screen Lock", lock),
        ui::section("Location", {ui::toggleRow("Location Services", gx::Key("org.gnome.system.location", "enabled"),
                                               "Apps can determine where you are, with your permission")}),
        ui::section("Camera & Microphone",
                    {ui::toggleRow("Camera Access", gx::Key(kPrivacy, "disable-camera"), "Apps can use the camera", true),
                     ui::toggleRow("Microphone Access", gx::Key(kPrivacy, "disable-microphone"), "Apps can use the microphone", true)}),
        ui::section("File History",
                    {ui::toggleRow("File History", gx::Key(kPrivacy, "remember-recent-files"), "Remember recently used files"),
                     ui::pickerRow("Keep History", gx::Key(kPrivacy, "recent-files-max-age"),
                                   {{"1 day", "1"}, {"7 days", "7"}, {"30 days", "30"}, {"Forever", "-1"}}),
                     FormRow(HStack({Spacer(), clearHistory}))}),
        ui::section("Trash & Temporary Files",
                    {ui::toggleRow("Automatically Delete Trash Content", gx::Key(kPrivacy, "remove-old-trash-files")),
                     ui::toggleRow("Automatically Delete Temporary Files", gx::Key(kPrivacy, "remove-old-temp-files")),
                     ui::pickerRow("Delete After", gx::Key(kPrivacy, "old-files-age"),
                                   {{"1 hour", "0"}, {"1 day", "1"}, {"2 days", "2"}, {"3 days", "3"}, {"7 days", "7"},
                                    {"14 days", "14"}, {"30 days", "30"}}),
                     FormRow(HStack({Spacer(), emptyTrash}))}),
        ui::section("Diagnostics", {ui::toggleRow("Automatic Problem Reporting", gx::Key(kPrivacy, "report-technical-problems"),
                                                  "Send reports of technical problems to the distributor")}),
    });
}

// Wellbeing -------------------------------------------------------------------------------------------------

namespace {

/// Membership of `value` in a string-list key as a switch.
FormRow listToggleRow(const std::string& title, const gx::Key& key, const std::string& value, const std::string& subtitle) {
    auto has = [key, value] {
        auto v = key.getStrv();
        return std::find(v.begin(), v.end(), value) != v.end();
    };
    auto t = Toggle("", has()).onChange([key, value](bool on) {
        auto v = key.getStrv();
        v.erase(std::remove(v.begin(), v.end(), value), v.end());
        if (on) v.push_back(value);
        key.setStrv(v);
    });
    key.watch([t, has]() mutable { t.setOn(has()); });
    if (!key) t.disabled();
    return FormRow(title, t, subtitle);
}

}  // namespace

Element wellbeingPanel(Context& /*ctx*/) {
    constexpr const char* kLimits = "org.gnome.desktop.screen-time-limits";
    constexpr const char* kBreaks = "org.gnome.desktop.break-reminders";
    if (!gx::hasSchema(kLimits) && !gx::hasSchema(kBreaks))
        return ui::unavailable(Icon::Hourglass, "Wellbeing Isn't Available", "Screen time and break reminders need GNOME 48 or later.");
    std::vector<ui::Option> limits;
    for (int m : {30, 60, 90, 120, 180, 240, 300, 360, 480}) limits.push_back({ui::formatDuration(m * 60), std::to_string(m * 60)});
    std::vector<ui::Option> intervals;
    for (int m : {10, 15, 20, 30, 45, 60, 90, 120}) intervals.push_back({"Every " + ui::formatDuration(m * 60), std::to_string(m * 60)});
    gx::Key selected(kBreaks, "selected-breaks");
    return ui::page({
        ui::hero(Icon::Hourglass, theme().colors.teal, "Wellbeing", "Healthy screen habits: daily limits and regular breaks."),
        ui::section("Screen Time",
                    {ui::toggleRow("Screen Time History", gx::Key(kLimits, "history-enabled"), "Record how long the screen is used"),
                     ui::toggleRow("Daily Limit", gx::Key(kLimits, "daily-limit-enabled"), "A reminder when the limit is reached"),
                     ui::pickerRow("Limit", gx::Key(kLimits, "daily-limit-seconds"), limits),
                     ui::toggleRow("Grayscale", gx::Key(kLimits, "grayscale"), "Turn the screen gray once the limit is reached")}),
        ui::section("Break Reminders",
                    {listToggleRow("Eyesight Reminders", selected, "eyesight", "Look away from the screen regularly"),
                     ui::pickerRow("Eyesight Breaks", gx::Key(kBreaks + std::string(".eyesight"), "interval-seconds"), intervals),
                     listToggleRow("Movement Reminders", selected, "movement", "Get up and move around"),
                     ui::pickerRow("Movement Breaks", gx::Key(kBreaks + std::string(".movement"), "interval-seconds"), intervals),
                     ui::toggleRow("Sounds", gx::Key(kBreaks + std::string(".movement"), "play-sound"), "Play a sound when a break starts")}),
    });
}

// Region & Language -----------------------------------------------------------------------------------------

namespace {

constexpr const char* kAccounts = "org.freedesktop.Accounts";
constexpr const char* kAccountsUser = "org.freedesktop.Accounts.User";

/// "tr_TR.utf8" -> "Turkish (Türkiye)" using iso-codes when installed.
class LocaleNames {
public:
    LocaleNames() {
        load("/usr/share/iso-codes/json/iso_639-2.json", "639-2", "alpha_2", m_languages);
        load("/usr/share/iso-codes/json/iso_639-3.json", "639-3", "alpha_3", m_languages);
        load("/usr/share/iso-codes/json/iso_3166-1.json", "3166-1", "alpha_2", m_countries);
    }

    [[nodiscard]] std::string name(const std::string& locale) const {
        const std::string base = locale.substr(0, locale.find_first_of(".@"));
        const std::string lang = base.substr(0, base.find('_'));
        const std::string country = base.find('_') != std::string::npos ? base.substr(base.find('_') + 1) : "";
        std::string out = lookup(m_languages, lang, lang);
        if (!country.empty()) out += " (" + lookup(m_countries, country, country) + ")";
        return out;
    }

private:
    static std::string lookup(const std::map<std::string, std::string>& m, const std::string& k, const std::string& fallback) {
        auto it = m.find(k);
        return it == m.end() ? fallback : it->second;
    }
    static void load(const char* path, const char* root, const char* codeKey, std::map<std::string, std::string>& out) {
        std::ifstream in(path);
        if (!in) return;
        std::stringstream ss;
        ss << in.rdbuf();
        const json::Value doc = json::Value::parse(ss.str());
        for (const auto& e : doc[root].array()) {
            const std::string code = e[codeKey].string();
            if (code.empty() || out.contains(code)) continue;
            out[code] = e["common_name"].str(e["name"].string());
        }
    }
    std::map<std::string, std::string> m_languages;
    std::map<std::string, std::string> m_countries;
};

/// Installed UTF-8 locales, normalised to "xx_YY.UTF-8".
std::vector<std::string> installedLocales() {
    std::vector<std::string> out;
    for (const auto& line : ui::split(gx::runSync({"locale", "-a"}).out, '\n')) {
        const std::string l = ui::trim(line);
        const std::string low = ui::lower(l);
        if (l.find('_') == std::string::npos || (low.find(".utf8") == std::string::npos && low.find(".utf-8") == std::string::npos)) continue;
        out.push_back(l.substr(0, l.find('.')) + ".UTF-8" + (l.find('@') != std::string::npos ? l.substr(l.find('@')) : ""));
    }
    std::sort(out.begin(), out.end());
    out.erase(std::unique(out.begin(), out.end()), out.end());
    return out;
}

std::string currentUserPath() {
    auto r = gx::callSync(gx::systemBus(), kAccounts, "/org/freedesktop/Accounts", kAccounts, "FindUserByName",
                          g_variant_new("(s)", g_get_user_name()), "(o)");
    if (!r) return {};
    const char* path = nullptr;
    g_variant_get(r.get(), "(&o)", &path);
    return path;
}

std::string userProperty(const std::string& path, const char* prop) {
    auto v = gx::property(gx::systemBus(), kAccounts, path.c_str(), kAccountsUser, prop);
    return v ? gx::str(v.get()) : "";
}

}  // namespace

Element regionPanel(Context& ctx) {
    auto names = std::make_shared<LocaleNames>();
    const auto locales = installedLocales();
    const std::string user = currentUserPath();
    std::vector<ui::Option> options;
    for (const auto& l : locales) options.push_back({names->name(l) + "  ·  " + l, l});

    auto languageLabel = Label("").secondary();
    auto syncLanguage = [user, names, languageLabel]() mutable {
        std::string lang = user.empty() ? "" : userProperty(user, "Language");
        if (lang.empty()) {
            const char* env = std::getenv("LANG");
            lang = env ? env : "C";
        }
        languageLabel.setText(names->name(lang));
    };
    syncLanguage();
    auto changeLanguage = Button("Change…");
    changeLanguage.onClick([anchor = ui::Anchor(changeLanguage), options, user, syncLanguage, appAnchor = ctx.anchor]() mutable {
        ui::chooseFromList(anchor, options, [user, syncLanguage, appAnchor](const std::string& l) mutable {
            if (user.empty()) return;
            gx::perform(gx::systemBus(), kAccounts, user.c_str(), kAccountsUser, "SetLanguage", g_variant_new("(s)", l.c_str()),
                        [syncLanguage, appAnchor](gx::Variant, const std::string& e) mutable {
                            if (!e.empty()) ui::message(appAnchor, "Couldn't change the language", e);
                            syncLanguage();
                        });
        }, "Search Languages");
    });

    // Plasma keeps the formats locale in plasma-localerc (applied at the next login, like GNOME).
    gx::Key region = desk::kde() ? kcfg::key("plasma-localerc", "Formats", "LANG", "s", "", kcfg::Reload::None, true)
                                 : gx::Key("org.gnome.system.locale", "region");
    auto formatsLabel = Label("").secondary();
    auto syncFormats = [region, names, formatsLabel]() mutable {
        const std::string r = region.getString();
        formatsLabel.setText(r.empty() ? "Same as Language" : names->name(r));
    };
    syncFormats();
    region.watch(syncFormats);
    auto changeFormats = Button("Change…");
    changeFormats.onClick([anchor = ui::Anchor(changeFormats), options, region] {
        auto items = options;
        items.insert(items.begin(), {"Same as Language", ""});
        ui::chooseFromList(anchor, items, [region](const std::string& l) { region.setString(l); }, "Search Formats");
    });

    // A sample of the chosen formats, rendered with the C library's locale data.
    auto preview = Label("").font(TextStyle::Subheadline).secondary();
    auto syncPreview = [region, preview]() mutable {
        std::string loc = region.getString();
        if (loc.empty()) loc = std::getenv("LANG") ? std::getenv("LANG") : "C";
        locale_t l = newlocale(LC_ALL_MASK, loc.c_str(), static_cast<locale_t>(nullptr));
        if (!l) l = newlocale(LC_ALL_MASK, (loc.substr(0, loc.find('.')) + ".utf8").c_str(), static_cast<locale_t>(nullptr));
        if (!l) return preview.setText({});
        const std::time_t now = std::time(nullptr);
        std::tm tm{};
        localtime_r(&now, &tm);
        char date[128];
        char time[64];
        strftime_l(date, sizeof date, "%x", &tm, l);
        strftime_l(time, sizeof time, "%X", &tm, l);
        freelocale(l);
        preview.setText(std::string(date) + " · " + time);
    };
    syncPreview();
    region.watch(syncPreview);

    std::vector<Element> sections{
        ui::section("Your Account",
                    {FormRow("Language", HStack({languageLabel, changeLanguage}).spacing(8).align(Alignment::Center),
                             "Takes effect the next time you log in"),
                     FormRow("Formats", HStack({formatsLabel, changeFormats}).spacing(8).align(Alignment::Center),
                             "Dates, times, numbers and currency"),
                     FormRow(preview)}),
    };
    // System-wide settings (login screen, new users) live in systemd-localed.
    if (auto sys = gx::properties(gx::systemBus(), "org.freedesktop.locale1", "/org/freedesktop/locale1", "org.freedesktop.locale1")) {
        std::string systemLang;
        for (const auto& e : gx::strv(gx::lookup(sys.get(), "Locale").get())) {
            if (e.rfind("LANG=", 0) == 0) systemLang = e.substr(5);
        }
        std::string layout = gx::lookupString(sys.get(), "X11Layout");
        if (const auto variant = gx::lookupString(sys.get(), "X11Variant"); !variant.empty()) layout += " (" + variant + ")";
        auto apply = Button("Use My Settings");
        apply.onClick([anchor = ui::Anchor(apply), user, region, appAnchor = ctx.anchor] {
            std::string lang = user.empty() ? "" : userProperty(user, "Language");
            if (lang.empty()) return;
            const std::string formats = region.getString().empty() ? lang : region.getString();
            ui::confirm(anchor, "Change the system language?",
                        "The login screen and new accounts will use your language and formats. This needs administrator rights.",
                        "Change", [lang, formats, appAnchor] {
                            const std::string a = "LANG=" + lang;
                            const std::string b = "LC_TIME=" + formats;
                            const std::string c = "LC_NUMERIC=" + formats;
                            const std::string d = "LC_MONETARY=" + formats;
                            const std::string e = "LC_MEASUREMENT=" + formats;
                            const char* entries[] = {a.c_str(), b.c_str(), c.c_str(), d.c_str(), e.c_str(), nullptr};
                            gx::perform(gx::systemBus(), "org.freedesktop.locale1", "/org/freedesktop/locale1", "org.freedesktop.locale1",
                                        "SetLocale", g_variant_new("(^asb)", entries, TRUE),
                                        [appAnchor](gx::Variant, const std::string& err) {
                                            if (!err.empty()) ui::message(appAnchor, "Couldn't change the system language", err);
                                        });
                        }, false);
        });
        sections.push_back(ui::section("Login Screen",
                                       {ui::infoRow("Language", systemLang.empty() ? "Not set" : names->name(systemLang) + " · " + systemLang),
                                        ui::infoRow("Keyboard Layout", layout.empty() ? "Not set" : layout),
                                        FormRow(HStack({Spacer(), apply}))},
                                       "Used before anyone logs in and for new users."));
    }
    return ui::page(sections);
}

// Date & Time ------------------------------------------------------------------------------------------------

namespace {

constexpr const char* kTimedate = "org.freedesktop.timedate1";
constexpr const char* kTimedatePath = "/org/freedesktop/timedate1";

class DateTimePanel {
public:
    explicit DateTimePanel(Context& ctx) : m_app(ctx.app), m_anchor(ctx.anchor) {
        m_ntp.onChange([this](bool on) {
            gx::perform(gx::systemBus(), kTimedate, kTimedatePath, kTimedate, "SetNTP", g_variant_new("(bb)", on, TRUE),
                        [this](gx::Variant, const std::string& e) {
                            if (!e.empty()) ui::message(m_anchor, "Couldn't change automatic time", e);
                            refresh();
                        });
        });
        m_zoneButton.onClick([this] { chooseZone(); });
        m_setTime.onClick([this] { setTime(); });
        gx::subscribe(gx::systemBus(), kTimedate, "org.freedesktop.DBus.Properties", "PropertiesChanged", kTimedatePath,
                      [this](const std::string&, const std::string&, GVariant*) { refresh(); });
        tick();
    }

    std::vector<Element> sections() {
        constexpr const char* kIface = "org.gnome.desktop.interface";
        if (desk::kde()) {
            // Plasma's clock format lives in each clock widget's own settings; formats follow the region.
            return {
                ui::section({}, {FormRow(HStack({VStack({m_clock, m_date}).spacing(2)}))}),
                ui::section("Date & Time",
                            {FormRow("Automatic Date & Time", m_ntp, "Requires internet access"),
                             FormRow("Time Zone", HStack({m_zoneLabel, m_zoneButton}).spacing(8).align(Alignment::Center)),
                             m_manualRow},
                            "The clock format follows Region & Language; right-click the panel clock for more options."),
            };
        }
        return {
            ui::section({}, {FormRow(HStack({VStack({m_clock, m_date}).spacing(2)}))}),
            ui::section("Date & Time",
                        {FormRow("Automatic Date & Time", m_ntp, "Requires internet access"),
                         ui::toggleRow("Automatic Time Zone", gx::Key("org.gnome.desktop.datetime", "automatic-timezone"),
                                       "Requires location services"),
                         FormRow("Time Zone", HStack({m_zoneLabel, m_zoneButton}).spacing(8).align(Alignment::Center)),
                         m_manualRow}),
            ui::section("Clock & Calendar",
                        {ui::pickerRow("Time Format", gx::Key(kIface, "clock-format"), {{"24-hour", "24h"}, {"AM / PM", "12h"}}),
                         ui::toggleRow("Week Day", gx::Key(kIface, "clock-show-weekday"), "In the top bar clock"),
                         ui::toggleRow("Date", gx::Key(kIface, "clock-show-date"), "In the top bar clock"),
                         ui::toggleRow("Seconds", gx::Key(kIface, "clock-show-seconds"), "In the top bar clock"),
                         ui::toggleRow("Week Numbers", gx::Key("org.gnome.desktop.calendar", "show-weekdate"), "In the calendar"),
                         ui::pickerRow("First Day of the Week", gx::Key("org.gnome.desktop.calendar", "week-start-day"),
                                       {{"Language Default", "default"}, {"Monday", "monday"}, {"Sunday", "sunday"},
                                        {"Saturday", "saturday"}})}),
        };
    }

    void refresh() {
        auto p = gx::properties(gx::systemBus(), kTimedate, kTimedatePath, kTimedate);
        if (!p) {
            m_ntp.setDisabled(true);
            m_zoneButton.setDisabled(true);
            m_zoneLabel.setText("timedated is not available");
            return;
        }
        m_timezone = gx::lookupString(p.get(), "Timezone");
        m_zoneLabel.setText(m_timezone);
        const bool ntp = gx::lookupBool(p.get(), "NTP");
        m_ntp.isOn(ntp);
        m_ntp.setDisabled(!gx::lookupBool(p.get(), "CanNTP"));
        m_manualRow.setHidden(ntp);
        tick(false);
    }

    void chooseZone() {
        gx::call(gx::systemBus(), kTimedate, kTimedatePath, kTimedate, "ListTimezones", nullptr,
                 [this](gx::Variant reply, const std::string& error) {
                     if (!reply) return ui::message(m_anchor, "Couldn't list time zones", error);
                     gx::Variant list(g_variant_get_child_value(reply.get(), 0));
                     std::vector<ui::Option> items;
                     for (const auto& z : gx::strv(list.get())) {
                         std::string label = z;
                         std::replace(label.begin(), label.end(), '_', ' ');
                         items.push_back({label, z});
                     }
                     ui::chooseFromList(ui::Anchor(m_zoneButton), items, [this](const std::string& zone) {
                         gx::perform(gx::systemBus(), kTimedate, kTimedatePath, kTimedate, "SetTimezone",
                                     g_variant_new("(sb)", zone.c_str(), TRUE), [this](gx::Variant, const std::string& e) {
                                         if (!e.empty()) ui::message(m_anchor, "Couldn't change the time zone", e);
                                         refresh();
                                     });
                     }, "Search Cities and Regions");
                 });
    }

private:
    void tick(bool reschedule = true) {
        const std::time_t now = std::time(nullptr);
        std::tm tm{};
        localtime_r(&now, &tm);
        char clock[32];
        char date[96];
        std::strftime(clock, sizeof clock, "%H:%M", &tm);
        std::strftime(date, sizeof date, "%A, %e %B %Y", &tm);
        m_clock.setText(clock);
        m_date.setText(date);
        if (!m_timeField->isFocused()) {
            char manual[32];
            std::strftime(manual, sizeof manual, "%Y-%m-%d %H:%M", &tm);
            m_timeField.setText(manual);
        }
        if (reschedule) m_app.after(60 - tm.tm_sec + 0.05, [this] { tick(); });
    }

    void setTime() {
        std::tm tm{};
        if (!strptime(m_timeField.value().c_str(), "%Y-%m-%d %H:%M", &tm)) {
            ui::message(ui::Anchor(m_setTime), "Invalid date", "Use the form 2025-01-31 18:30.");
            return;
        }
        tm.tm_isdst = -1;
        const std::time_t t = std::mktime(&tm);
        const auto usec = static_cast<gint64>(t) * 1000000;
        gx::perform(gx::systemBus(), kTimedate, kTimedatePath, kTimedate, "SetTime", g_variant_new("(xbb)", usec, FALSE, TRUE),
                    [this](gx::Variant, const std::string& e) {
                        if (!e.empty()) ui::message(m_anchor, "Couldn't set the time", e);
                        tick(false);
                    });
    }

    App& m_app;
    Element m_anchor;
    std::string m_timezone;
    Label m_clock = Label("").font(TextStyle::LargeTitle).weight(FontWeight::Semibold);
    Label m_date = Label("").secondary();
    Toggle m_ntp{"", false};
    Label m_zoneLabel = Label("").secondary();
    Button m_zoneButton = Button("Change…");
    TextField m_timeField = TextField("YYYY-MM-DD HH:MM").width(170);
    Button m_setTime = Button("Set");
    FormRow m_manualRow{"Date & Time", HStack({m_timeField, m_setTime}).spacing(6).align(Alignment::Center)};
};

}  // namespace

Element dateTimePanel(Context& ctx) {
    auto panel = std::make_shared<DateTimePanel>(ctx);
    panel->refresh();
    if (ctx.demo == "zones") ctx.app.after(0.6, [p = panel.get()] { p->chooseZone(); });
    return ui::page(panel->sections(), panel);
}

// Users -------------------------------------------------------------------------------------------------------

namespace {

/// Circular avatar: the account picture, or initials on a tinted disc.
Canvas avatar(std::shared_ptr<ImageData> image, const std::string& name, float size) {
    std::string initials;
    for (const auto& part : ui::split(name, ' ')) {
        if (!part.empty() && initials.size() < 2) initials += static_cast<char>(std::toupper(static_cast<unsigned char>(part[0])));
    }
    auto c = Canvas([image, initials](Painter& p, const Rect& r) {
        const float radius = r.width * 0.5f;
        if (image) {
            p.drawImage(r, *image, radius);
        } else {
            p.fillVerticalGradient(r, radius, theme().colors.gray.mix(Color::white(), 0.2f), theme().colors.gray);
            p.drawText(initials, r, theme().font(TextStyle::Title1).withSize(r.height * 0.38f).withWeight(FontWeight::Semibold),
                       Color::white(), TextAlign::Center);
        }
    });
    c.frame(size, size);
    return c;
}

class UsersPanel {
public:
    explicit UsersPanel(Context& ctx) : m_app(ctx.app), m_window(ctx.window), m_anchor(ctx.anchor), m_path(currentUserPath()) {
        if (!m_path.empty()) {
            gx::subscribe(gx::systemBus(), kAccounts, nullptr, "Changed", m_path.c_str(),
                          [this](const std::string&, const std::string&, GVariant*) { refresh(); });
        }
    }

    [[nodiscard]] bool available() const { return !m_path.empty(); }

    std::vector<Element> sections() {
        m_nameField.onSubmit([this](const std::string& name) { setName(name); });
        m_saveName.onClick([this] { setName(m_nameField.value()); });
        m_changePicture.onClick([this] { choosePicture(); });
        m_autoLogin.onChange([this](bool on) {
            gx::perform(gx::systemBus(), kAccounts, m_path.c_str(), kAccountsUser, "SetAutomaticLogin", g_variant_new("(b)", on),
                        [this](gx::Variant, const std::string& e) {
                            if (!e.empty()) ui::message(m_anchor, "Couldn't change automatic login", e);
                            refresh();
                        });
        });
        return {m_header,
                ui::section("Account",
                            {FormRow("Name", HStack({m_nameField, m_saveName}).spacing(6).align(Alignment::Center)), m_typeRow,
                             FormRow("Automatic Login", m_autoLogin, "Log in without a password at startup"),
                             FormRow("Password", Label("Change it with “passwd” in a terminal").secondary())}),
                m_others.element()};
    }

    void refresh() {
        auto p = gx::properties(gx::systemBus(), kAccounts, m_path.c_str(), kAccountsUser);
        if (!p) return;
        const std::string real = gx::lookupString(p.get(), "RealName");
        const std::string login = gx::lookupString(p.get(), "UserName");
        const std::string icon = gx::lookupString(p.get(), "IconFile");
        const bool admin = gx::lookupInt(p.get(), "AccountType") == 1;
        std::error_code ec;
        std::shared_ptr<ImageData> picture;
        if (!icon.empty() && fs::exists(icon, ec)) picture = ImageData::load(icon, 256);
        m_header.clear();
        m_header.add(HStack({avatar(picture, real.empty() ? login : real, 84),
                             VStack({Label(real.empty() ? login : real).font(TextStyle::Title2).weight(FontWeight::Semibold),
                                     Label(login + (admin ? " · Administrator" : " · Standard account")).secondary(), m_changePicture})
                                 .spacing(4)})
                         .spacing(16)
                         .align(Alignment::Leading)
                         .padding(Insets(4, 4)));
        if (!m_nameField->isFocused()) m_nameField.setText(real);
        m_type.setText(admin ? "Administrator" : "Standard");
        m_autoLogin.isOn(gx::lookupBool(p.get(), "AutomaticLogin"));

        std::vector<Element> rows;
        if (auto list = gx::callSync(gx::systemBus(), kAccounts, "/org/freedesktop/Accounts", kAccounts, "ListCachedUsers", nullptr, "(ao)")) {
            gx::Variant paths(g_variant_get_child_value(list.get(), 0));
            GVariantIter it;
            g_variant_iter_init(&it, paths.get());
            const char* path = nullptr;
            while (g_variant_iter_next(&it, "&o", &path)) {
                if (path == m_path) continue;
                auto up = gx::properties(gx::systemBus(), kAccounts, path, kAccountsUser);
                if (!up || gx::lookupBool(up.get(), "SystemAccount")) continue;
                const std::string n = gx::lookupString(up.get(), "RealName");
                const std::string u = gx::lookupString(up.get(), "UserName");
                const std::string ic = gx::lookupString(up.get(), "IconFile");
                std::shared_ptr<ImageData> img;
                if (!ic.empty() && fs::exists(ic, ec)) img = ImageData::load(ic, 96);
                rows.push_back(FormRow(HStack({avatar(img, n.empty() ? u : n, 32),
                                               VStack({Label(n.empty() ? u : n), Label(u).font(TextStyle::Subheadline).secondary()}).spacing(1),
                                               Spacer(),
                                               Label(gx::lookupInt(up.get(), "AccountType") == 1 ? "Administrator" : "Standard").secondary()})
                                           .spacing(10)
                                           .align(Alignment::Center)));
            }
        }
        m_others.setRows(rows);
    }

private:
    void setName(const std::string& name) {
        if (name.empty()) return;
        gx::perform(gx::systemBus(), kAccounts, m_path.c_str(), kAccountsUser, "SetRealName", g_variant_new("(s)", name.c_str()),
                    [this](gx::Variant, const std::string& e) {
                        if (!e.empty()) ui::message(m_anchor, "Couldn't change your name", e);
                        refresh();
                    });
    }

    void choosePicture() {
        m_app.openFiles({{"Images", "png;jpg;jpeg"}}, false, [this](std::vector<std::string> paths) {
            if (paths.empty()) return;
            // AccountsService copies the file (and scales it); it must be readable by the daemon.
            gx::perform(gx::systemBus(), kAccounts, m_path.c_str(), kAccountsUser, "SetIconFile",
                        g_variant_new("(s)", paths[0].c_str()), [this](gx::Variant, const std::string& e) {
                            if (!e.empty()) ui::message(m_anchor, "Couldn't change your picture", e);
                            refresh();
                        });
        }, &m_window);
    }

    App& m_app;
    Window& m_window;
    Element m_anchor;
    std::string m_path;
    VStack m_header = VStack({});
    TextField m_nameField = TextField("Full Name").width(220);
    Button m_saveName = Button("Save");
    Button m_changePicture = Button("Change Picture…").controlSize(ControlSize::Small);
    Label m_type = Label("").secondary();
    FormRow m_typeRow = ui::infoRow("Account Type", m_type);
    Toggle m_autoLogin{"", false};
    ui::DynamicSection m_others{"Other Users", "No other users."};
};

}  // namespace

Element usersPanel(Context& ctx) {
    auto panel = std::make_shared<UsersPanel>(ctx);
    if (!panel->available())
        return ui::unavailable(Icon::PersonCircle, "Accounts Service Isn't Running",
                               "User settings need the accountsservice daemon.");
    auto sections = panel->sections();
    panel->refresh();
    return ui::page(sections, panel);
}

// About -------------------------------------------------------------------------------------------------------

namespace {

std::map<std::string, std::string> osRelease() {
    std::map<std::string, std::string> out;
    for (const char* path : {"/etc/os-release", "/usr/lib/os-release"}) {
        std::ifstream in(path);
        if (!in) continue;
        std::string line;
        while (std::getline(in, line)) {
            const size_t eq = line.find('=');
            if (eq == std::string::npos) continue;
            std::string v = line.substr(eq + 1);
            if (v.size() >= 2 && (v.front() == '"' || v.front() == '\'')) v = v.substr(1, v.size() - 2);
            out[line.substr(0, eq)] = v;
        }
        break;
    }
    return out;
}

std::string cpuName() {
    std::ifstream in("/proc/cpuinfo");
    std::string line;
    std::string model;
    int count = 0;
    while (std::getline(in, line)) {
        if (line.rfind("model name", 0) == 0) {
            ++count;
            if (model.empty()) model = ui::trim(line.substr(line.find(':') + 1));
        }
    }
    if (model.empty()) return "Unknown";
    return model + " × " + std::to_string(count);
}

std::string memorySize() {
    std::ifstream in("/proc/meminfo");
    std::string key;
    long kb = 0;
    in >> key >> kb;
    return ui::formatBytes(static_cast<double>(kb) * 1024);
}

std::string graphicsNames() {
    // switcheroo-control knows every GPU by its marketing name.
    std::string out;
    if (auto gpus = gx::property(gx::systemBus(), "net.hadess.SwitcherooControl", "/net/hadess/SwitcherooControl",
                                 "net.hadess.SwitcherooControl", "GPUs")) {
        GVariantIter it;
        g_variant_iter_init(&it, gpus.get());
        GVariant* dict = nullptr;
        while (g_variant_iter_next(&it, "@a{sv}", &dict)) {
            gx::Variant d(dict);
            const std::string name = gx::lookupString(dict, "Name");
            if (!name.empty()) out += (out.empty() ? "" : "\n") + name;
        }
    }
    if (out.empty()) {
        for (const auto& line : ui::split(gx::runSync({"lspci"}).out, '\n')) {
            if (line.find("VGA") != std::string::npos || line.find("3D controller") != std::string::npos ||
                line.find("Display controller") != std::string::npos) {
                const size_t c = line.find(": ");
                if (c != std::string::npos) out += (out.empty() ? "" : "\n") + line.substr(c + 2);
            }
        }
    }
    return out.empty() ? "Unknown" : out;
}

std::string diskCapacity() {
    struct statvfs st {};
    if (statvfs("/", &st) != 0) return "Unknown";
    const double total = static_cast<double>(st.f_blocks) * static_cast<double>(st.f_frsize);
    const double avail = static_cast<double>(st.f_bavail) * static_cast<double>(st.f_frsize);
    return ui::formatBytes(total) + " (" + ui::formatBytes(avail) + " available)";
}

std::string sanitizeHostname(const std::string& pretty) {
    std::string out;
    for (char c : pretty) {
        if (std::isalnum(static_cast<unsigned char>(c))) out += static_cast<char>(std::tolower(static_cast<unsigned char>(c)));
        else if ((c == ' ' || c == '-' || c == '_') && !out.empty() && out.back() != '-') out += '-';
    }
    while (!out.empty() && out.back() == '-') out.pop_back();
    return out.substr(0, 63);
}

}  // namespace

Element aboutPanel(Context& ctx) {
    constexpr const char* kHostname = "org.freedesktop.hostname1";
    constexpr const char* kHostnamePath = "/org/freedesktop/hostname1";
    auto os = osRelease();
    auto host = gx::properties(gx::systemBus(), kHostname, kHostnamePath, kHostname);
    std::string pretty = gx::lookupString(host.get(), "PrettyHostname");
    if (pretty.empty()) pretty = gx::lookupString(host.get(), "Hostname", g_get_host_name());
    std::string model = ui::trim(gx::lookupString(host.get(), "HardwareVendor") + " " + gx::lookupString(host.get(), "HardwareModel"));
    if (model.empty()) {
        // Older systemd: read the firmware's DMI strings directly.
        auto dmi = [](const char* f) {
            std::ifstream in(std::string("/sys/class/dmi/id/") + f);
            std::string s;
            std::getline(in, s);
            return ui::trim(s);
        };
        model = ui::trim(dmi("sys_vendor") + " " + dmi("product_name"));
    }

    auto nameField = TextField("Device Name").text(pretty).width(240);
    nameField.onSubmit([appAnchor = ctx.anchor](const std::string& name) {
        if (name.empty()) return;
        gx::perform(gx::systemBus(), kHostname, kHostnamePath, kHostname, "SetPrettyHostname", g_variant_new("(sb)", name.c_str(), TRUE),
                    [appAnchor, name](gx::Variant, const std::string& e) {
                        if (!e.empty()) return ui::message(appAnchor, "Couldn't rename the device", e);
                        if (auto h = sanitizeHostname(name); !h.empty())
                            gx::perform(gx::systemBus(), kHostname, kHostnamePath, kHostname, "SetStaticHostname",
                                        g_variant_new("(sb)", h.c_str(), TRUE));
                    });
    });

    std::string shell;
    if (desk::kde()) shell = kde::plasmaVersion();
    else if (auto v = gx::property(gx::sessionBus(), "org.gnome.Shell", "/org/gnome/Shell", "org.gnome.Shell", "ShellVersion")) shell = gx::str(v.get());
    struct utsname un {};
    uname(&un);
    const char* session = std::getenv("XDG_SESSION_TYPE");
    std::string windowing = session ? session : "Unknown";
    if (!windowing.empty()) windowing[0] = static_cast<char>(std::toupper(static_cast<unsigned char>(windowing[0])));
    const std::string osName = os.contains("PRETTY_NAME") ? os["PRETTY_NAME"] : os["NAME"];

    // The details are also offered as text for bug reports.
    std::vector<std::pair<std::string, std::string>> hardware{
        {"Hardware Model", model.empty() ? "Unknown" : model}, {"Memory", memorySize()}, {"Processor", cpuName()},
        {"Graphics", graphicsNames()}, {"Disk Capacity", diskCapacity()}};
    std::vector<std::pair<std::string, std::string>> software{
        {"OS Name", osName}, {"OS Type", sizeof(void*) == 8 ? "64-bit" : "32-bit"},
        {desk::kde() ? "KDE Plasma Version" : "GNOME Version", shell.empty() ? "Not running" : shell}, {"Windowing System", windowing},
        {"Kernel Version", std::string(un.sysname) + " " + un.release}};
    if (const auto fw = gx::lookupString(host.get(), "FirmwareVersion"); !fw.empty()) software.emplace_back("Firmware Version", fw);

    std::vector<Element> hwRows;
    std::vector<Element> swRows;
    std::string text;
    for (const auto& [k, v] : hardware) {
        hwRows.push_back(ui::infoRow(k, v));
        text += k + ": " + v + "\n";
    }
    for (const auto& [k, v] : software) {
        swRows.push_back(ui::infoRow(k, v));
        text += k + ": " + v + "\n";
    }
    auto copy = Button(Icon::DocOnDoc, "Copy System Details");
    copy.onClick([text, anchor = ui::Anchor(copy)] {
        const Element e = anchor;
        if (e && e.widget()->host()) e.widget()->host()->setClipboardText(text);
    });

    return ui::page({
        ui::hero(Icon::Display, theme().colors.gray, osName.empty() ? "GNU/Linux" : osName,
                 os.contains("HOME_URL") ? os["HOME_URL"] : std::string(un.nodename)),
        ui::section({}, {FormRow("Device Name", nameField, "Shown to other devices on the network")}),
        ui::section("Hardware", hwRows),
        ui::section("Software", swRows),
        HStack({Spacer(), copy}),
    });
}
