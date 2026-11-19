// Reads desktop preferences from the xdg-desktop-portal Settings interface (GNOME, KDE and others).
//
// libdbus is loaded at run time, like SDL does, so it is not a build dependency: without it (or without a
// portal) the system accent colour is simply unknown. The dark/light scheme comes from SDL itself.
#include <dlfcn.h>

#include <cstdint>
#include <optional>
#include <type_traits>

#include "platform.hpp"

namespace mayo::platform {

namespace {

// The few libdbus declarations needed, so no headers are required at build time.
struct DBusError {
    const char* name;
    const char* message;
    unsigned int dummy;  // five one-bit fields in the real struct
    void* padding;
};
struct alignas(8) DBusMessageIter {
    unsigned char opaque[128];  // the real struct is 72 bytes on 64-bit targets
};
struct DBusConnection;
struct DBusMessage;
constexpr int kTypeInvalid = 0;
constexpr int kTypeString = 's';
constexpr int kTypeVariant = 'v';
constexpr int kTypeStruct = 'r';
constexpr int kTypeDouble = 'd';
constexpr int kBusSession = 0;

struct DBus {
    void (*errorInit)(DBusError*);
    void (*errorFree)(DBusError*);
    DBusConnection* (*busGet)(int, DBusError*);
    void (*setExitOnDisconnect)(DBusConnection*, unsigned);
    DBusMessage* (*newMethodCall)(const char*, const char*, const char*, const char*);
    unsigned (*appendArgs)(DBusMessage*, int, ...);
    DBusMessage* (*sendWithReplyAndBlock)(DBusConnection*, DBusMessage*, int, DBusError*);
    void (*messageUnref)(DBusMessage*);
    unsigned (*iterInit)(DBusMessage*, DBusMessageIter*);
    int (*iterArgType)(DBusMessageIter*);
    void (*iterRecurse)(DBusMessageIter*, DBusMessageIter*);
    void (*iterGetBasic)(DBusMessageIter*, void*);
    unsigned (*iterNext)(DBusMessageIter*);
    DBusConnection* connection = nullptr;

    static DBus* get() {
        static DBus* instance = [] () -> DBus* {
            void* lib = dlopen("libdbus-1.so.3", RTLD_NOW | RTLD_LOCAL);
            if (!lib) return nullptr;
            static DBus d{};
            bool ok = true;
            auto sym = [&](auto& fn, const char* name) {
                fn = reinterpret_cast<std::remove_reference_t<decltype(fn)>>(dlsym(lib, name));
                ok = ok && fn;
            };
            sym(d.errorInit, "dbus_error_init");
            sym(d.errorFree, "dbus_error_free");
            sym(d.busGet, "dbus_bus_get");
            sym(d.setExitOnDisconnect, "dbus_connection_set_exit_on_disconnect");
            sym(d.newMethodCall, "dbus_message_new_method_call");
            sym(d.appendArgs, "dbus_message_append_args");
            sym(d.sendWithReplyAndBlock, "dbus_connection_send_with_reply_and_block");
            sym(d.messageUnref, "dbus_message_unref");
            sym(d.iterInit, "dbus_message_iter_init");
            sym(d.iterArgType, "dbus_message_iter_get_arg_type");
            sym(d.iterRecurse, "dbus_message_iter_recurse");
            sym(d.iterGetBasic, "dbus_message_iter_get_basic");
            sym(d.iterNext, "dbus_message_iter_next");
            if (!ok) return nullptr;
            DBusError err;
            d.errorInit(&err);
            d.connection = d.busGet(kBusSession, &err);
            d.errorFree(&err);
            if (!d.connection) return nullptr;
            // The shared connection would otherwise exit() the process if the bus goes away.
            d.setExitOnDisconnect(d.connection, 0);
            return &d;
        }();
        return instance;
    }

    /// Calls Settings.ReadOne (or the older Read) and leaves `iter` on the value inside the variant(s).
    DBusMessage* read(const char* ns, const char* key, DBusMessageIter& iter) {
        for (const char* method : {"ReadOne", "Read"}) {
            DBusMessage* msg = newMethodCall("org.freedesktop.portal.Desktop", "/org/freedesktop/portal/desktop",
                                             "org.freedesktop.portal.Settings", method);
            if (!msg) return nullptr;
            appendArgs(msg, kTypeString, &ns, kTypeString, &key, kTypeInvalid);
            DBusError err;
            errorInit(&err);
            // Short timeout: this runs on the UI thread, and a missing portal must not stall startup.
            DBusMessage* reply = sendWithReplyAndBlock(connection, msg, 250, &err);
            errorFree(&err);
            messageUnref(msg);
            if (!reply) continue;
            if (!iterInit(reply, &iter)) {
                messageUnref(reply);
                continue;
            }
            // Read() wraps the value in a second variant.
            while (iterArgType(&iter) == kTypeVariant) {
                DBusMessageIter inner;
                iterRecurse(&iter, &inner);
                iter = inner;
            }
            return reply;
        }
        return nullptr;
    }
};

}  // namespace

std::optional<Color> Platform::systemAccentColor() {
    DBus* d = DBus::get();
    if (!d) return std::nullopt;
    DBusMessageIter iter;
    DBusMessage* reply = d->read("org.freedesktop.appearance", "accent-color", iter);
    if (!reply) return std::nullopt;
    std::optional<Color> out;
    if (d->iterArgType(&iter) == kTypeStruct) {
        DBusMessageIter field;
        d->iterRecurse(&iter, &field);
        double rgb[3] = {-1, -1, -1};
        for (double& v : rgb) {
            if (d->iterArgType(&field) != kTypeDouble) break;
            d->iterGetBasic(&field, &v);
            d->iterNext(&field);
        }
        // Out-of-range components mean "no accent colour set".
        bool valid = true;
        for (double v : rgb) valid = valid && v >= 0 && v <= 1;
        if (valid) out = Color{static_cast<float>(rgb[0]), static_cast<float>(rgb[1]), static_cast<float>(rgb[2]), 1};
    }
    d->messageUnref(reply);
    return out;
}

}  // namespace mayo::platform
