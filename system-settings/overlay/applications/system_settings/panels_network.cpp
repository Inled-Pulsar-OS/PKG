// Wi-Fi, Network and Bluetooth.
//
// Wi-Fi and wired state comes from nmcli (terse mode is stable and scriptable) and is refreshed whenever
// `nmcli monitor` reports a change. Joining a protected network goes through NetworkManager's D-Bus API
// instead, so the password never appears on a command line. Bluetooth talks to BlueZ directly.
#include <algorithm>
#include <map>
#include <set>

#include "glib_support.hpp"
#include "kconfig.hpp"
#include "panels.hpp"
#include "ui.hpp"

using namespace mayo;

namespace {

constexpr const char* kNM = "org.freedesktop.NetworkManager";
constexpr const char* kNMPath = "/org/freedesktop/NetworkManager";

/// Splits one line of `nmcli -t` output: fields are ':'-separated, with '\:' and '\\' escaped.
std::vector<std::string> terseFields(const std::string& line) {
    std::vector<std::string> out(1);
    for (size_t i = 0; i < line.size(); ++i) {
        if (line[i] == '\\' && i + 1 < line.size()) {
            out.back() += line[++i];
        } else if (line[i] == ':') {
            out.emplace_back();
        } else {
            out.back() += line[i];
        }
    }
    return out;
}

std::vector<std::vector<std::string>> terseLines(const std::string& text) {
    std::vector<std::vector<std::string>> out;
    for (const auto& l : ui::split(text, '\n')) {
        if (!l.empty()) out.push_back(terseFields(l));
    }
    return out;
}

/// "nmcli" plus arguments.
std::vector<std::string> nmcli(std::initializer_list<std::string> args) {
    std::vector<std::string> v{"nmcli"};
    v.insert(v.end(), args);
    return v;
}

void reportFailure(const Element& anchor, const std::string& title, const gx::ProcessResult& r) {
    if (r.ok()) return;
    std::string msg = ui::trim(r.err.empty() ? r.out : r.err);
    if (msg.rfind("Error: ", 0) == 0) msg = msg.substr(7);
    ui::message(anchor, title, msg.empty() ? "NetworkManager reported an error." : msg);
}

bool networkManagerRunning() {
    if (!gx::haveProgram("nmcli")) return false;
    return ui::trim(gx::runSync(nmcli({"-t", "-f", "RUNNING", "general"})).out) == "running";
}

}  // namespace

// Wi-Fi ----------------------------------------------------------------------------------------------------

namespace {

struct AccessPoint {
    std::string ssid;
    int signal = 0;
    std::string security;
    bool inUse = false;
    std::string frequency;
};

struct KnownNetwork {
    std::string name;
    std::string uuid;
    std::string ssid;
    bool autoconnect = true;
    bool active = false;
};

struct WifiState {
    bool enabled = false;
    std::string iface;
    std::string deviceState;
    std::vector<AccessPoint> aps;
    std::vector<KnownNetwork> known;
    bool scanning = false;
};

bool isEnterprise(const std::string& security) { return security.find("802.1X") != std::string::npos; }
bool isOpen(const std::string& security) { return security.empty() || security == "--" || security == "OWE"; }

/// NetworkManager key-mgmt for a security string from nmcli.
std::string keyManagement(const std::string& security) {
    if (security.find("WEP") != std::string::npos) return "none";
    if (security.find("WPA3") != std::string::npos && security.find("WPA2") == std::string::npos &&
        security.find("WPA1") == std::string::npos)
        return "sae";
    return "wpa-psk";
}

GVariant* ssidBytes(const std::string& ssid) {
    return g_variant_new_fixed_array(G_VARIANT_TYPE_BYTE, ssid.data(), ssid.size(), 1);
}

/// Joins a protected (or hidden) network through D-Bus: AddAndActivateConnection with just the secret; NM
/// fills in the rest from the access point.
void joinWithPassword(const std::string& iface, const std::string& ssid, const std::string& security,
                      const std::string& password, bool hidden, std::function<void(const std::string&)> done) {
    std::string err;
    auto dev = gx::callSync(gx::systemBus(), kNM, kNMPath, kNM, "GetDeviceByIpIface", g_variant_new("(s)", iface.c_str()),
                            "(o)", &err);
    if (!dev) return done(err);
    const char* devPath = nullptr;
    g_variant_get(dev.get(), "(&o)", &devPath);

    std::string apPath = "/";
    if (!hidden) {
        int best = -1;
        auto aps = gx::property(gx::systemBus(), kNM, devPath, "org.freedesktop.NetworkManager.Device.Wireless", "AccessPoints");
        GVariantIter it;
        const char* ap = nullptr;
        if (aps) {
            g_variant_iter_init(&it, aps.get());
            while (g_variant_iter_next(&it, "&o", &ap)) {
                auto props = gx::properties(gx::systemBus(), kNM, ap, "org.freedesktop.NetworkManager.AccessPoint");
                auto raw = gx::lookup(props.get(), "Ssid");
                if (!raw) continue;
                gsize n = 0;
                const auto* bytes = static_cast<const char*>(g_variant_get_fixed_array(raw.get(), &n, 1));
                if (std::string(bytes, n) != ssid) continue;
                const int strength = static_cast<int>(gx::lookupInt(props.get(), "Strength"));
                if (strength > best) {
                    best = strength;
                    apPath = ap;
                }
            }
        }
    }

    GVariantBuilder conn;
    g_variant_builder_init(&conn, G_VARIANT_TYPE("a{sa{sv}}"));
    if (hidden) {
        GVariantBuilder c;
        g_variant_builder_init(&c, G_VARIANT_TYPE("a{sv}"));
        g_variant_builder_add(&c, "{sv}", "id", g_variant_new_string(ssid.c_str()));
        g_variant_builder_add(&c, "{sv}", "type", g_variant_new_string("802-11-wireless"));
        g_variant_builder_add(&conn, "{sa{sv}}", "connection", &c);
        GVariantBuilder w;
        g_variant_builder_init(&w, G_VARIANT_TYPE("a{sv}"));
        g_variant_builder_add(&w, "{sv}", "ssid", ssidBytes(ssid));
        g_variant_builder_add(&w, "{sv}", "hidden", g_variant_new_boolean(TRUE));
        g_variant_builder_add(&conn, "{sa{sv}}", "802-11-wireless", &w);
    }
    if (!password.empty()) {
        GVariantBuilder sec;
        g_variant_builder_init(&sec, G_VARIANT_TYPE("a{sv}"));
        const std::string km = keyManagement(security);
        g_variant_builder_add(&sec, "{sv}", "key-mgmt", g_variant_new_string(km.c_str()));
        if (km == "none") {
            g_variant_builder_add(&sec, "{sv}", "wep-key0", g_variant_new_string(password.c_str()));
            g_variant_builder_add(&sec, "{sv}", "wep-key-type", g_variant_new_uint32(1));
        } else {
            g_variant_builder_add(&sec, "{sv}", "psk", g_variant_new_string(password.c_str()));
        }
        g_variant_builder_add(&conn, "{sa{sv}}", "802-11-wireless-security", &sec);
    }
    gx::perform(gx::systemBus(), kNM, kNMPath, kNM, "AddAndActivateConnection",
                g_variant_new("(a{sa{sv}}oo)", &conn, devPath, apPath.c_str()),
                [done](gx::Variant, const std::string& e) { done(e); });
}

/// Password popover for a protected network (or SSID + password for a hidden one).
void askPassword(const Element& anchor, const std::string& iface, const std::string& ssid, const std::string& security,
                 const Element& errorAnchor) {
    const bool hidden = ssid.empty();
    auto name = TextField("Network Name").width(260);
    auto pw = TextField("Password").secure().width(260);
    auto securityPicker = Picker({"WPA/WPA2 Personal", "WPA3 Personal", "None"}, 0);
    auto join = Button("Join").style(ButtonStyle::Prominent);
    auto cancel = Button("Cancel");
    auto content = VStack({Label(hidden ? "Join a hidden network" : "Enter the password for “" + ssid + "”").font(TextStyle::Headline).maxWidth(280)})
                       .spacing(10)
                       .width(290);
    if (hidden) {
        content.add(name);
        content.add(HStack({Label("Security").secondary(), Spacer(), securityPicker}).align(Alignment::Center));
    }
    content.add(pw);
    content.add(HStack({Spacer(), cancel, join}).spacing(8));
    auto pop = Popover(content);
    std::weak_ptr<PopoverView> weak = pop.node();
    auto submit = [weak, iface, ssid, security, name, pw, securityPicker, hidden, errorAnchor] {
        const std::string net = hidden ? name.value() : ssid;
        if (net.empty()) return;
        const int sec = securityPicker.selectedIndex();
        const std::string secName = hidden ? (sec == 0 ? "WPA2" : sec == 1 ? "WPA3" : "") : security;
        if (auto p = weak.lock()) p->dismiss();
        joinWithPassword(iface, net, secName, secName.empty() ? std::string() : pw.value(), hidden,
                         [errorAnchor, net](const std::string& e) {
                             if (!e.empty()) ui::message(errorAnchor, "Couldn't join “" + net + "”", e);
                         });
    };
    join.onClick(submit);
    pw.onSubmit([submit](const std::string&) { submit(); });
    cancel.onClick([weak] {
        if (auto p = weak.lock()) p->dismiss();
    });
    pop.show(anchor);
    if (auto* h = (hidden ? name : pw)->host()) h->setFocus(hidden ? name.node().get() : pw.node().get(), false);
}

void showWifiDetails(const Element& anchor, const std::string& iface, const KnownNetwork* known, const AccessPoint& ap) {
    auto r = gx::runSync(nmcli({"-t", "-f", "IP4.ADDRESS,IP4.GATEWAY,IP4.DNS,IP6.ADDRESS,GENERAL.HWADDR", "device", "show", iface}));
    std::map<std::string, std::string> info;
    for (const auto& f : terseLines(r.out)) {
        if (f.size() < 2) continue;
        std::string key = f[0].substr(0, f[0].find('['));
        std::string value = f[1];
        for (size_t i = 2; i < f.size(); ++i) value += ":" + f[i];  // IPv6 / MAC colons
        if (!info[key].empty()) info[key] += ", ";
        info[key] += value;
    }
    auto grid = VStack({}).spacing(4);
    auto line = [&](const std::string& k, const std::string& v) {
        if (!v.empty()) grid.add(HStack({Label(k).secondary().width(110), Label(v).maxWidth(240)}).spacing(8));
    };
    line("Signal", std::to_string(ap.signal) + "%");
    line("Security", isOpen(ap.security) ? "None" : ap.security);
    line("Frequency", ap.frequency);
    line("IPv4 Address", info["IP4.ADDRESS"]);
    line("Router", info["IP4.GATEWAY"]);
    line("DNS", info["IP4.DNS"]);
    line("IPv6 Address", info["IP6.ADDRESS"]);
    line("Hardware Address", info["GENERAL.HWADDR"]);
    auto content = VStack({Label(ap.ssid).font(TextStyle::Headline), Divider(), grid}).spacing(8).width(380);
    if (known) {
        auto autoJoin = Toggle("Connect automatically", known->autoconnect).onChange([uuid = known->uuid](bool on) {
            gx::run(nmcli({"connection", "modify", "uuid", uuid, "connection.autoconnect", on ? "yes" : "no"}));
        });
        content.add(Divider());
        content.add(autoJoin);
    }
    Popover(content).show(anchor);
}

}  // namespace

namespace {

class WifiPanel {
public:
    explicit WifiPanel(Element anchor) : m_anchor(std::move(anchor)) {
        m_others.setAccessory(HStack({m_scanButton, m_otherButton}).spacing(6));
        m_scanButton.onClick([this] { scan(); });
        m_otherButton.onClick([this] { askPassword(ui::Anchor(m_otherButton), m_state.iface, {}, {}, m_anchor); });
        m_toggle.onChange([this](bool on) {
            gx::run(nmcli({"radio", "wifi", on ? "on" : "off"}), [this](const gx::ProcessResult& r) {
                reportFailure(m_anchor, "Couldn't change Wi-Fi", r);
                refresh();
            });
        });
        gx::stream(nmcli({"monitor"}), [changed = m_changed](const std::string&) mutable { changed(); });
    }

    std::vector<Element> sections() {
        std::vector<Element> top{FormRow(HStack({ui::tile(Icon::Wifi, theme().colors.blue, 28),
                                                 VStack({Label("Wi-Fi").weight(FontWeight::Medium), m_status}).spacing(1),
                                                 Spacer(), m_spinner, m_toggle})
                                             .spacing(10)
                                             .align(Alignment::Center))};
        if (auto airplane = airplaneRow()) top.push_back(airplane);
        return {ui::section({}, top), m_known.element(), m_others.element(), m_saved.element()};
    }

    /// Four quick reads, chained so the UI is rebuilt once from consistent data.
    void refresh() {
        if (m_busy) {
            m_again = true;
            return;
        }
        m_busy = true;
        gx::run(nmcli({"-t", "-f", "WIFI", "general"}), [this](const gx::ProcessResult& radio) {
            m_state.enabled = ui::trim(radio.out) == "enabled";
            gx::run(nmcli({"-t", "-f", "DEVICE,TYPE,STATE", "device"}), [this](const gx::ProcessResult& devs) {
                m_state.iface.clear();
                for (const auto& f : terseLines(devs.out)) {
                    if (f.size() >= 3 && f[1] == "wifi") {
                        m_state.iface = f[0];
                        m_state.deviceState = f[2];
                        break;
                    }
                }
                gx::run(nmcli({"-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY,FREQ", "device", "wifi", "list", "--rescan", "no"}),
                        [this](const gx::ProcessResult& list) {
                            parseAccessPoints(list.out);
                            gx::run(nmcli({"-t", "-f", "NAME,UUID,TYPE,AUTOCONNECT,ACTIVE", "connection", "show"}),
                                    [this](const gx::ProcessResult& conns) { parseConnections(conns.out); }, false);
                        }, false);
            }, false);
        }, false);
    }

    void joinHidden() { askPassword(ui::Anchor(m_otherButton), m_state.iface, {}, {}, m_anchor); }

    void scan() {
        if (m_state.iface.empty()) {
            m_scanWhenReady = true;  // the first refresh hasn't found the device yet
            return;
        }
        if (m_state.scanning || !m_state.enabled) return;
        m_state.scanning = true;
        rebuild();
        gx::run(nmcli({"device", "wifi", "rescan"}), [this](const gx::ProcessResult&) {
            // The scan itself finishes a few seconds after the request returns.
            App::current().after(3.0, [this] {
                m_state.scanning = false;
                refresh();
            });
        }, false);
    }

private:
    Element airplaneRow() {
        // Airplane mode lives in gnome-settings-daemon (it drives rfkill for every radio).
        static constexpr const char* kRfkill = "org.gnome.SettingsDaemon.Rfkill";
        static constexpr const char* kPath = "/org/gnome/SettingsDaemon/Rfkill";
        auto airplane = gx::property(gx::sessionBus(), kRfkill, kPath, kRfkill, "AirplaneMode");
        if (!airplane) return Element();
        auto toggle = Toggle("", g_variant_get_boolean(airplane.get()) != FALSE).onChange([](bool on) {
            gx::setProperty(gx::sessionBus(), kRfkill, kPath, kRfkill, "AirplaneMode", g_variant_new_boolean(on));
        });
        gx::subscribe(gx::sessionBus(), kRfkill, "org.freedesktop.DBus.Properties", "PropertiesChanged", kPath,
                      [t = std::weak_ptr<ToggleView>(toggle.node())](const std::string&, const std::string&, GVariant*) {
                          auto v = gx::property(gx::sessionBus(), kRfkill, kPath, kRfkill, "AirplaneMode");
                          if (auto n = t.lock(); n && v) n->setOn(g_variant_get_boolean(v.get()) != FALSE);
                      });
        return FormRow("Airplane Mode", toggle, "Turns off Wi-Fi, Bluetooth and mobile broadband");
    }

    void parseAccessPoints(const std::string& out) {
        // One row per network name: the strongest access point, or the one in use.
        std::map<std::string, AccessPoint> best;
        for (const auto& f : terseLines(out)) {
            if (f.size() < 5 || f[1].empty()) continue;  // hidden networks have no name
            AccessPoint ap{f[1], std::atoi(f[2].c_str()), f[3], f[0] == "*", f[4]};
            auto& b = best[ap.ssid];
            if (b.ssid.empty() || ap.inUse || (!b.inUse && ap.signal > b.signal)) b = ap;
        }
        m_state.aps.clear();
        for (auto& [_, ap] : best) m_state.aps.push_back(ap);
        std::sort(m_state.aps.begin(), m_state.aps.end(), [](const AccessPoint& a, const AccessPoint& b) {
            return a.inUse != b.inUse ? a.inUse : a.signal > b.signal;
        });
    }

    void parseConnections(const std::string& out) {
        m_state.known.clear();
        std::vector<std::string> args{"nmcli", "-t", "-f", "connection.uuid,802-11-wireless.ssid", "connection", "show"};
        for (const auto& f : terseLines(out)) {
            if (f.size() < 5 || f[2] != "802-11-wireless") continue;
            m_state.known.push_back({f[0], f[1], f[0], f[3] == "yes", f[4] == "yes"});
            args.push_back(f[1]);
        }
        if (m_state.known.empty()) return finish();
        // The profile name usually is the SSID, but not always.
        gx::run(args, [this](const gx::ProcessResult& ssids) {
            std::string uuid;
            for (const auto& f : terseLines(ssids.out)) {
                if (f.size() < 2) continue;
                std::string value = f[1];
                for (size_t i = 2; i < f.size(); ++i) value += ":" + f[i];
                if (f[0] == "connection.uuid") uuid = value;
                if (f[0] == "802-11-wireless.ssid") {
                    for (auto& k : m_state.known) {
                        if (k.uuid == uuid) k.ssid = value;
                    }
                }
            }
            std::sort(m_state.known.begin(), m_state.known.end(),
                      [](const KnownNetwork& a, const KnownNetwork& b) { return ui::lower(a.name) < ui::lower(b.name); });
            finish();
        }, false);
    }

    void finish() {
        rebuild();
        m_busy = false;
        if (m_scanWhenReady && !m_state.iface.empty()) {
            m_scanWhenReady = false;
            scan();
        }
        if (m_again) {
            m_again = false;
            refresh();
        }
    }

    [[nodiscard]] const KnownNetwork* knownFor(const std::string& ssid) const {
        for (const auto& k : m_state.known) {
            if (k.ssid == ssid) return &k;
        }
        return nullptr;
    }

    void afterCommand(const std::string& failureTitle, const gx::ProcessResult& r) {
        reportFailure(m_anchor, failureTitle, r);
        refresh();
    }

    Element networkRow(const AccessPoint& ap) {
        const KnownNetwork* k = knownFor(ap.ssid);
        std::optional<KnownNetwork> known;
        if (k) known = *k;
        auto row = ui::ClickRow({ui::signalIcon(ap.signal, !isOpen(ap.security)), Label(ap.ssid).lineLimit(1)});
        if (ap.inUse) row.add(Label("Connected").font(TextStyle::Subheadline).secondary());
        row.add(Spacer());
        const std::string iface = m_state.iface;
        if (ap.inUse || known) {
            auto info = Button(Icon::Info).style(ButtonStyle::Borderless).tooltip("Details");
            info.onClick([anchor = ui::Anchor(info), iface, known, ap] {
                showWifiDetails(anchor, iface, known ? &*known : nullptr, ap);
            });
            row.add(info);
        }
        if (ap.inUse) {
            row.add(Button("Disconnect").controlSize(ControlSize::Small).onClick([this, iface] {
                gx::run(nmcli({"device", "disconnect", iface}),
                        [this](const gx::ProcessResult& r) { afterCommand("Couldn't disconnect", r); });
            }));
            return row;
        }
        row.onClick([this, anchor = ui::Anchor(row), iface, ap, known] {
            if (isEnterprise(ap.security) && !known) {
                ui::message(anchor, "Enterprise network",
                            "Networks that need a username and certificate (802.1X) can be set up with nm-connection-editor.");
                return;
            }
            auto after = [this, title = "Couldn't join “" + ap.ssid + "”"](const gx::ProcessResult& r) { afterCommand(title, r); };
            if (known) gx::run(nmcli({"connection", "up", "uuid", known->uuid}), after);
            else if (isOpen(ap.security)) gx::run(nmcli({"device", "wifi", "connect", ap.ssid, "ifname", iface}), after);
            else askPassword(anchor, iface, ap.ssid, ap.security, m_anchor);
        });
        return row;
    }

    Element savedRow(const KnownNetwork& k) {
        auto forget = Button("Forget").controlSize(ControlSize::Small);
        forget.onClick([this, anchor = ui::Anchor(forget), k] {
            ui::confirm(anchor, "Forget “" + k.name + "”?",
                        "Your computer will no longer join this network automatically, and its password is removed.", "Forget",
                        [this, uuid = k.uuid] {
                            gx::run(nmcli({"connection", "delete", "uuid", uuid}),
                                    [this](const gx::ProcessResult& r) { afterCommand("Couldn't forget the network", r); });
                        });
        });
        auto autoJoin = Toggle("Auto-Join", k.autoconnect).onChange([uuid = k.uuid](bool on) {
            gx::run(nmcli({"connection", "modify", "uuid", uuid, "connection.autoconnect", on ? "yes" : "no"}));
        });
        return FormRow(HStack({ui::glyph(Icon::Wifi), Label(k.name).lineLimit(1), Spacer(), autoJoin, forget})
                           .spacing(10)
                           .align(Alignment::Center));
    }

    void rebuild() {
        const WifiState& s = m_state;
        m_toggle.isOn(s.enabled);
        m_toggle.setDisabled(s.iface.empty());
        m_spinner.setHidden(!s.scanning);
        m_scanButton.setDisabled(!s.enabled || s.scanning);
        m_otherButton.setDisabled(!s.enabled);
        const AccessPoint* current = nullptr;
        for (const auto& ap : s.aps) {
            if (ap.inUse) current = &ap;
        }
        if (s.iface.empty()) m_status.setText("No Wi-Fi adapter found");
        else if (!s.enabled) m_status.setText("Off");
        else if (current) m_status.setText("Connected to “" + current->ssid + "”");
        else if (s.deviceState.find("connecting") != std::string::npos) m_status.setText("Connecting…");
        else m_status.setText("Not connected");

        std::vector<Element> knownRows;
        std::vector<Element> otherRows;
        if (s.enabled) {
            for (const auto& ap : s.aps) (knownFor(ap.ssid) || ap.inUse ? knownRows : otherRows).push_back(networkRow(ap));
        }
        m_known.setRows(knownRows);
        m_others.setRows(otherRows);
        m_known.setHidden(!s.enabled);
        m_others.setHidden(!s.enabled);

        std::vector<Element> savedRows;
        for (const auto& k : s.known) savedRows.push_back(savedRow(k));
        m_saved.setRows(savedRows);
    }

    Element m_anchor;
    WifiState m_state;
    bool m_busy = false;
    bool m_again = false;
    bool m_scanWhenReady = false;
    gx::Debounce m_changed{0.4, [this] { refresh(); }};
    Toggle m_toggle{"", false};
    Label m_status = Label("").font(TextStyle::Subheadline).secondary();
    ProgressBar m_spinner = ProgressSpinner().hidden();
    Button m_scanButton = Button("Scan").controlSize(ControlSize::Small);
    Button m_otherButton = Button("Other…").controlSize(ControlSize::Small).tooltip("Join a hidden network");
    ui::DynamicSection m_known{"Known Networks", "No saved networks are in range."};
    ui::DynamicSection m_others{"Other Networks", "No networks found."};
    ui::DynamicSection m_saved{"Saved Networks", "No saved Wi-Fi networks.",
                               "Networks this computer has joined before. Forgetting one removes its password."};
};

}  // namespace

Element wifiPanel(Context& ctx) {
    if (!networkManagerRunning())
        return ui::unavailable(Icon::Wifi, "NetworkManager Isn't Running",
                               "Wi-Fi settings need NetworkManager and its nmcli tool.");
    auto panel = std::make_shared<WifiPanel>(ctx.anchor);
    ctx.onShown.emplace("wifi", [p = panel.get()] { p->scan(); });
    if (ctx.demo == "other") ctx.app.after(1.0, [p = panel.get()] { p->joinHidden(); });
    panel->refresh();
    return ui::page(panel->sections(), panel);
}

// Network (wired, VPN, proxy) ------------------------------------------------------------------------------

namespace {

std::string deviceStateText(const std::string& state) {
    if (state == "connected") return "Connected";
    if (state == "unavailable") return "Cable unplugged";
    if (state == "disconnected") return "Disconnected";
    if (state == "unmanaged") return "Not managed by NetworkManager";
    if (state.find("connecting") != std::string::npos) return "Connecting…";
    return state;
}

void showDeviceDetails(const Element& anchor, const std::string& title, const std::string& iface) {
    auto r = gx::runSync(nmcli({"-t", "-f", "GENERAL.HWADDR,CAPABILITIES.SPEED,IP4.ADDRESS,IP4.GATEWAY,IP4.DNS,IP6.ADDRESS",
                                "device", "show", iface}));
    std::map<std::string, std::string> info;
    for (const auto& f : terseLines(r.out)) {
        if (f.size() < 2) continue;
        std::string key = f[0].substr(0, f[0].find('['));
        std::string value = f[1];
        for (size_t i = 2; i < f.size(); ++i) value += ":" + f[i];
        if (!info[key].empty()) info[key] += ", ";
        info[key] += value;
    }
    auto grid = VStack({}).spacing(4);
    for (auto [label, key] : {std::pair{"Link Speed", "CAPABILITIES.SPEED"}, {"IPv4 Address", "IP4.ADDRESS"},
                              {"Router", "IP4.GATEWAY"}, {"DNS", "IP4.DNS"}, {"IPv6 Address", "IP6.ADDRESS"},
                              {"Hardware Address", "GENERAL.HWADDR"}}) {
        if (!info[key].empty() && info[key] != "unknown")
            grid.add(HStack({Label(label).secondary().width(120), Label(info[key]).maxWidth(240)}).spacing(8));
    }
    Popover(VStack({Label(title).font(TextStyle::Headline), Divider(), grid}).spacing(8).width(390)).show(anchor);
}

class NetworkPanel {
public:
    NetworkPanel(App& app, Window& window, Element anchor) : m_app(app), m_window(window), m_anchor(std::move(anchor)) {
        auto import = Button(Icon::Plus).style(ButtonStyle::Borderless).tooltip("Import VPN Configuration…");
        import.onClick([this] { importVpn(); });
        m_vpn.setAccessory(import);
        gx::stream(nmcli({"monitor"}), [changed = m_changed](const std::string&) mutable { changed(); });
    }

    std::vector<Element> sections() { return {m_wired.element(), m_vpn.element(), proxySection()}; }

    void refresh() {
        gx::run(nmcli({"-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device"}), [this](const gx::ProcessResult& devs) {
            gx::run(nmcli({"-t", "-f", "NAME,UUID,TYPE,ACTIVE", "connection", "show"}),
                    [this, devices = devs.out](const gx::ProcessResult& conns) { rebuild(devices, conns.out); }, false);
        }, false);
    }

private:
    void rebuild(const std::string& devices, const std::string& connections) {
        std::vector<Element> wired;
        for (const auto& f : terseLines(devices)) {
            if (f.size() < 4 || f[1] != "ethernet") continue;
            const std::string iface = f[0];
            const bool connected = f[2] == "connected";
            auto toggle = Toggle("", connected).onChange([this, iface](bool on) {
                gx::run(nmcli({"device", on ? "connect" : "disconnect", iface}), [this](const gx::ProcessResult& r) {
                    reportFailure(m_anchor, "Couldn't change the wired connection", r);
                    refresh();
                });
            });
            toggle.setDisabled(f[2] == "unavailable" || f[2] == "unmanaged");
            auto info = Button(Icon::Info).style(ButtonStyle::Borderless).tooltip("Details");
            info.onClick([anchor = ui::Anchor(info), iface] { showDeviceDetails(anchor, "Ethernet (" + iface + ")", iface); });
            std::string status = deviceStateText(f[2]);
            if (connected && !f[3].empty()) status += " · " + f[3];
            wired.push_back(FormRow(HStack({ui::tile(Icon::Network, theme().colors.blue, 26),
                                            VStack({Label("Ethernet (" + iface + ")"), Label(status).font(TextStyle::Subheadline).secondary()}).spacing(1),
                                            Spacer(), info, toggle})
                                        .spacing(10)
                                        .align(Alignment::Center)));
        }
        m_wired.setRows(wired);

        std::vector<Element> vpns;
        for (const auto& f : terseLines(connections)) {
            if (f.size() < 4 || (f[2] != "vpn" && f[2] != "wireguard")) continue;
            const std::string uuid = f[1];
            const std::string name = f[0];
            auto toggle = Toggle("", f[3] == "yes").onChange([this, uuid, name](bool on) {
                gx::run(nmcli({"connection", on ? "up" : "down", "uuid", uuid}), [this, name](const gx::ProcessResult& r) {
                    reportFailure(m_anchor, "Couldn't change “" + name + "”", r);
                    refresh();
                });
            });
            auto remove = Button(Icon::Trash).style(ButtonStyle::Borderless).tooltip("Remove");
            remove.onClick([this, anchor = ui::Anchor(remove), uuid, name] {
                ui::confirm(anchor, "Remove “" + name + "”?", "The VPN configuration is deleted from this computer.", "Remove",
                            [this, uuid] {
                                gx::run(nmcli({"connection", "delete", "uuid", uuid}), [this](const gx::ProcessResult& r) {
                                    reportFailure(m_anchor, "Couldn't remove the VPN", r);
                                    refresh();
                                });
                            });
            });
            vpns.push_back(FormRow(HStack({ui::tile(Icon::Lock, theme().colors.gray, 26), Label(name), 
                                           Label(f[2] == "wireguard" ? "WireGuard" : "VPN").font(TextStyle::Subheadline).secondary(),
                                           Spacer(), remove, toggle})
                                       .spacing(10)
                                       .align(Alignment::Center)));
        }
        m_vpn.setRows(vpns);
    }

    void importVpn() {
        m_app.openFiles({{"VPN Configurations", "ovpn;conf"}}, false, [this](std::vector<std::string> paths) {
            if (paths.empty()) return;
            const std::string& file = paths[0];
            const bool wireguard = file.size() > 5 && file.compare(file.size() - 5, 5, ".conf") == 0;
            gx::run(nmcli({"connection", "import", "type", wireguard ? "wireguard" : "openvpn", "file", file}),
                    [this](const gx::ProcessResult& r) {
                        reportFailure(m_anchor, "Couldn't import the VPN", r);
                        refresh();
                    });
        }, &m_window);
    }

    static FormRow hostPortRow(const std::string& title, const std::string& schema) {
        gx::Key host(schema, "host");
        gx::Key port(schema, "port");
        auto hostField = TextField("Host").text(host.getString()).width(200);
        auto portField = TextField("Port").text(port.getInt() ? std::to_string(port.getInt()) : "").width(64);
        hostField.onChange([host](const std::string& t) { host.setString(t); });
        portField.onChange([port](const std::string& t) { port.setInt(std::atoi(t.c_str())); });
        return FormRow(title, HStack({hostField, Label(":").secondary(), portField}).spacing(4).align(Alignment::Center));
    }

    Element proxySection() {
        if (desk::kde()) return kde::proxySection();
        constexpr const char* kProxy = "org.gnome.system.proxy";
        gx::Key mode(kProxy, "mode");
        if (!mode) return Element();
        gx::Key ignore(kProxy, "ignore-hosts");
        auto join = [](const std::vector<std::string>& v) {
            std::string s;
            for (const auto& h : v) s += (s.empty() ? "" : ", ") + h;
            return s;
        };
        auto ignoreField = TextField("localhost, 127.0.0.0/8").text(join(ignore.getStrv())).width(300);
        ignoreField.onChange([ignore](const std::string& t) {
            std::vector<std::string> hosts;
            for (const auto& h : ui::split(t, ',')) {
                if (auto v = ui::trim(h); !v.empty()) hosts.push_back(v);
            }
            ignore.setStrv(hosts);
        });
        auto manual = ui::section("Manual Proxy", {hostPortRow("HTTP Proxy", "org.gnome.system.proxy.http"),
                                                   hostPortRow("HTTPS Proxy", "org.gnome.system.proxy.https"),
                                                   hostPortRow("FTP Proxy", "org.gnome.system.proxy.ftp"),
                                                   hostPortRow("SOCKS Host", "org.gnome.system.proxy.socks"),
                                                   FormRow("Ignored Hosts", ignoreField)});
        auto automatic = ui::section("Automatic Proxy", {ui::textRow("Configuration URL", gx::Key(kProxy, "autoconfig-url"),
                                                                     "http://example.com/proxy.pac", 300)});
        auto sync = [mode, manual, automatic] {
            const std::string m = mode.getString();
            manual.widget()->setHidden(m != "manual");
            automatic.widget()->setHidden(m != "auto");
        };
        sync();
        mode.watch(sync);
        return VStack({ui::section("Proxy", {ui::pickerRow("Network Proxy", mode,
                                                           {{"Off", "none"}, {"Manual", "manual"}, {"Automatic", "auto"}})}),
                       manual, automatic})
            .spacing(22);
    }

    App& m_app;
    Window& m_window;
    Element m_anchor;
    gx::Debounce m_changed{0.4, [this] { refresh(); }};
    ui::DynamicSection m_wired{"Wired", "No wired network adapters."};
    ui::DynamicSection m_vpn{"VPN", "No VPN connections.", "OpenVPN (.ovpn) and WireGuard (.conf) files can be imported."};
};

}  // namespace

Element networkPanel(Context& ctx) {
    if (!networkManagerRunning())
        return ui::unavailable(Icon::Network, "NetworkManager Isn't Running",
                               "Network settings need NetworkManager and its nmcli tool.");
    auto panel = std::make_shared<NetworkPanel>(ctx.app, ctx.window, ctx.anchor);
    panel->refresh();
    return ui::page(panel->sections(), panel);
}

// Bluetooth ---------------------------------------------------------------------------------------------

namespace {

constexpr const char* kBluez = "org.bluez";

struct BtDevice {
    std::string path;
    std::string name;
    std::string icon;
    bool paired = false;
    bool connected = false;
    bool named = false;  ///< has a real name, not just an address
    int battery = -1;
};

Icon deviceIcon(const std::string& icon) {
    if (icon.rfind("audio", 0) == 0) return Icon::MusicNote;
    if (icon == "input-mouse" || icon == "input-tablet") return Icon::Mouse;
    if (icon == "input-keyboard") return Icon::Keyboard;
    if (icon == "computer") return Icon::Display;
    if (icon == "printer") return Icon::Printer;
    return Icon::Bluetooth;
}

class BluetoothPanel {
public:
    explicit BluetoothPanel(Element anchor) : m_anchor(std::move(anchor)) {
        m_toggle.onChange([this](bool on) { setPowered(on); });
        auto changed = m_changed;
        gx::subscribe(gx::systemBus(), kBluez, "org.freedesktop.DBus.ObjectManager", nullptr, nullptr,
                      [changed](const std::string&, const std::string&, GVariant*) mutable { changed(); });
        gx::subscribe(gx::systemBus(), kBluez, "org.freedesktop.DBus.Properties", "PropertiesChanged", nullptr,
                      [changed](const std::string&, const std::string&, GVariant*) mutable { changed(); });
    }

    std::vector<Element> sections() {
        return {ui::section({}, {FormRow(HStack({ui::tile(Icon::Bluetooth, theme().colors.blue, 28),
                                                 VStack({Label("Bluetooth").weight(FontWeight::Medium), m_status}).spacing(1),
                                                 Spacer(), m_spinner, m_toggle})
                                             .spacing(10)
                                             .align(Alignment::Center))}),
                m_mine.element(), m_nearby.element()};
    }

    void refresh() {
        gx::call(gx::systemBus(), kBluez, "/", "org.freedesktop.DBus.ObjectManager", "GetManagedObjects", nullptr,
                 [this](gx::Variant reply, const std::string& error) {
                     if (!reply) {
                         m_status.setText(error.empty() ? "Bluetooth service not available" : error);
                         m_toggle.setDisabled(true);
                         return;
                     }
                     parse(reply.get());
                     rebuild();
                 });
    }

    /// Looks for new devices while the panel is open (BlueZ stops when we ask, or when we exit).
    void discover() {
        if (m_adapter.empty() || !m_powered) {
            m_discoverWhenReady = true;
            return;
        }
        const int generation = ++m_discovery;
        gx::call(gx::systemBus(), kBluez, m_adapter.c_str(), "org.bluez.Adapter1", "StartDiscovery", nullptr, {});
        App::current().after(60, [this, generation] {
            if (generation == m_discovery)
                gx::call(gx::systemBus(), kBluez, m_adapter.c_str(), "org.bluez.Adapter1", "StopDiscovery", nullptr, {});
        });
    }

private:
    void parse(GVariant* reply) {
        gx::Variant objects(g_variant_get_child_value(reply, 0));
        m_devices.clear();
        m_adapter.clear();
        GVariantIter it;
        g_variant_iter_init(&it, objects.get());
        const char* path = nullptr;
        GVariant* ifaces = nullptr;
        std::map<std::string, int> batteries;
        while (g_variant_iter_next(&it, "{&o@a{sa{sv}}}", &path, &ifaces)) {
            gx::Variant owned(ifaces);
            gx::Variant adapter(g_variant_lookup_value(ifaces, "org.bluez.Adapter1", G_VARIANT_TYPE("a{sv}")));
            if (adapter && m_adapter.empty()) {
                m_adapter = path;
                m_powered = gx::lookupBool(adapter.get(), "Powered");
                m_discovering = gx::lookupBool(adapter.get(), "Discovering");
                m_alias = gx::lookupString(adapter.get(), "Alias");
            }
            gx::Variant battery(g_variant_lookup_value(ifaces, "org.bluez.Battery1", G_VARIANT_TYPE("a{sv}")));
            if (battery) batteries[path] = static_cast<int>(gx::lookupInt(battery.get(), "Percentage", -1));
            gx::Variant dev(g_variant_lookup_value(ifaces, "org.bluez.Device1", G_VARIANT_TYPE("a{sv}")));
            if (!dev) continue;
            BtDevice d;
            d.path = path;
            d.name = gx::lookupString(dev.get(), "Alias");
            d.icon = gx::lookupString(dev.get(), "Icon");
            d.paired = gx::lookupBool(dev.get(), "Paired");
            d.connected = gx::lookupBool(dev.get(), "Connected");
            d.named = gx::lookup(dev.get(), "Name") != nullptr;
            m_devices.push_back(std::move(d));
        }
        for (auto& d : m_devices) {
            if (auto b = batteries.find(d.path); b != batteries.end()) d.battery = b->second;
        }
        std::sort(m_devices.begin(), m_devices.end(), [](const BtDevice& a, const BtDevice& b) {
            return a.connected != b.connected ? a.connected : ui::lower(a.name) < ui::lower(b.name);
        });
        if (m_discoverWhenReady && m_powered) {
            m_discoverWhenReady = false;
            discover();
        }
    }

    void setPowered(bool on) {
        if (m_adapter.empty()) return;
        gx::setProperty(gx::systemBus(), kBluez, m_adapter.c_str(), "org.bluez.Adapter1", "Powered", g_variant_new_boolean(on),
                        [this, on](gx::Variant, const std::string& error) {
                            if ((error.find("Blocked") != std::string::npos || error.find("rfkill") != std::string::npos) &&
                                !gx::hasName(gx::sessionBus(), "org.gnome.SettingsDaemon.Rfkill")) {
                                // No GNOME daemon (Plasma): lift the soft block with rfkill(8), which the active
                                // session may use, then retry.
                                gx::run({"rfkill", on ? "unblock" : "block", "bluetooth"}, [this, on](const gx::ProcessResult&) {
                                    gx::setProperty(gx::systemBus(), kBluez, m_adapter.c_str(), "org.bluez.Adapter1", "Powered",
                                                    g_variant_new_boolean(on));
                                });
                            } else if (error.find("Blocked") != std::string::npos || error.find("rfkill") != std::string::npos) {
                                // Soft-blocked by rfkill: lift the block through gnome-settings-daemon, then retry.
                                gx::setProperty(gx::sessionBus(), "org.gnome.SettingsDaemon.Rfkill", "/org/gnome/SettingsDaemon/Rfkill",
                                                "org.gnome.SettingsDaemon.Rfkill", "BluetoothAirplaneMode", g_variant_new_boolean(!on),
                                                [this, on](gx::Variant, const std::string&) {
                                                    gx::setProperty(gx::systemBus(), kBluez, m_adapter.c_str(), "org.bluez.Adapter1",
                                                                    "Powered", g_variant_new_boolean(on));
                                                });
                            } else if (!error.empty()) {
                                ui::message(m_anchor, "Couldn't change Bluetooth", error);
                            }
                            refresh();
                        });
    }

    void deviceCall(const std::string& path, const char* method, const std::string& failure,
                    std::function<void()> then = {}) {
        m_busy.insert(path);
        rebuild();
        gx::perform(gx::systemBus(), kBluez, path.c_str(), "org.bluez.Device1", method, nullptr,
                    [this, path, failure, then](gx::Variant, const std::string& error) {
                        m_busy.erase(path);
                        if (!error.empty() && error.find("AlreadyExists") == std::string::npos &&
                            error.find("AlreadyConnected") == std::string::npos)
                            ui::message(m_anchor, failure, error);
                        else if (then)
                            then();
                        refresh();
                    });
    }

    void forget(const BtDevice& d) {
        gx::perform(gx::systemBus(), kBluez, m_adapter.c_str(), "org.bluez.Adapter1", "RemoveDevice",
                    g_variant_new("(o)", d.path.c_str()), [this](gx::Variant, const std::string& error) {
                        if (!error.empty()) ui::message(m_anchor, "Couldn't remove the device", error);
                        refresh();
                    });
    }

    Element deviceRow(const BtDevice& d) {
        std::string status = d.connected ? "Connected" : d.paired ? "Not Connected" : "";
        if (d.battery >= 0) status += (status.empty() ? "" : " · ") + std::to_string(d.battery) + "% battery";
        auto text = VStack({Label(d.name).lineLimit(1)}).spacing(1);
        if (!status.empty()) text.add(Label(status).font(TextStyle::Subheadline).secondary());
        auto row = ui::ClickRow({ui::glyph(deviceIcon(d.icon), 18, false), text, Spacer()});
        if (m_busy.contains(d.path)) row.add(ProgressSpinner());
        const std::string path = d.path;
        if (d.paired) {
            const bool connected = d.connected;
            row.onClick([this, path, connected] {
                deviceCall(path, connected ? "Disconnect" : "Connect", connected ? "Couldn't disconnect" : "Couldn't connect");
            });
            const BtDevice copy = d;
            auto anchor = std::make_shared<Element>();
            row.add(Menu("", {MenuItem(connected ? "Disconnect" : "Connect",
                                       [this, path, connected] {
                                           deviceCall(path, connected ? "Disconnect" : "Connect",
                                                      connected ? "Couldn't disconnect" : "Couldn't connect");
                                       }),
                              MenuItem::separator(),
                              MenuItem("Forget This Device…", [this, copy] {
                                  ui::confirm(m_anchor, "Forget “" + copy.name + "”?",
                                              "You'll need to pair it again to use it with this computer.", "Forget",
                                              [this, copy] { forget(copy); });
                              }).asDestructive()})
                        .icon(Icon::Ellipsis)
                        .borderless());
        } else {
            row.onClick([this, path] {
                deviceCall(path, "Pair", "Couldn't pair", [this, path] {
                    gx::setProperty(gx::systemBus(), kBluez, path.c_str(), "org.bluez.Device1", "Trusted", g_variant_new_boolean(TRUE));
                    deviceCall(path, "Connect", "Paired, but couldn't connect");
                });
            });
            row.add(Label("Pair").foreground(theme().colors.accent));
        }
        return row;
    }

    void rebuild() {
        m_toggle.isOn(m_powered);
        m_toggle.setDisabled(m_adapter.empty());
        m_spinner.setHidden(!(m_powered && m_discovering));
        if (m_adapter.empty()) m_status.setText("No Bluetooth adapter found");
        else if (!m_powered) m_status.setText("Off");
        else m_status.setText("Discoverable as “" + m_alias + "”");
        std::vector<Element> mine;
        std::vector<Element> nearby;
        if (m_powered) {
            for (const auto& d : m_devices) {
                if (d.paired) mine.push_back(deviceRow(d));
                else if (d.named) nearby.push_back(deviceRow(d));
            }
        }
        m_mine.setRows(mine);
        m_nearby.setRows(nearby);
        m_mine.setHidden(!m_powered);
        m_nearby.setHidden(!m_powered);
    }

    Element m_anchor;
    std::string m_adapter;
    std::string m_alias;
    bool m_powered = false;
    bool m_discovering = false;
    bool m_discoverWhenReady = false;
    int m_discovery = 0;
    std::vector<BtDevice> m_devices;
    std::set<std::string> m_busy;
    gx::Debounce m_changed{0.3, [this] { refresh(); }};
    Toggle m_toggle{"", false};
    Label m_status = Label("").font(TextStyle::Subheadline).secondary();
    ProgressBar m_spinner = ProgressSpinner().hidden();
    ui::DynamicSection m_mine{"My Devices", "No devices are paired with this computer."};
    ui::DynamicSection m_nearby{"Nearby Devices", "Looking for devices…",
                                "Devices appear here when they are in pairing mode."};
};

}  // namespace

Element bluetoothPanel(Context& ctx) {
    if (!gx::hasName(gx::systemBus(), kBluez))
        return ui::unavailable(Icon::Bluetooth, "Bluetooth Isn't Available",
                               "The BlueZ service (bluetoothd) isn't installed or running.");
    auto panel = std::make_shared<BluetoothPanel>(ctx.anchor);
    ctx.onShown.emplace("bluetooth", [p = panel.get()] { p->discover(); });
    panel->refresh();
    return ui::page(panel->sections(), panel);
}
