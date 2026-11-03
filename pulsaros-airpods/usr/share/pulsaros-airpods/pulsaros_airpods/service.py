"""Session-bus service org.pulsaros.AirPods, exported with GDBus from the contract XML."""

import logging
import os
from typing import Callable, Dict

from gi.repository import Gio, GLib

from .controller import Snapshot

log = logging.getLogger(__name__)

BUS_NAME = "org.pulsaros.AirPods"
OBJECT_PATH = "/org/pulsaros/AirPods"
IFACE = "org.pulsaros.AirPods"
PROPERTIES_IFACE = "org.freedesktop.DBus.Properties"

# D-Bus property name -> (Snapshot attribute, GVariant type)
SNAPSHOT_PROPERTIES = {
    "Connected": ("connected", "b"),
    "Name": ("name", "s"),
    "Address": ("address", "s"),
    "ModelId": ("model_id", "s"),
    "Model": ("model", "s"),
    "IsHeadphones": ("is_headphones", "b"),
    "HasAdvertisement": ("has_advertisement", "b"),
    "Left": ("left", "i"),
    "Right": ("right", "i"),
    "Case": ("case", "i"),
    "LeftCharging": ("left_charging", "b"),
    "RightCharging": ("right_charging", "b"),
    "CaseCharging": ("case_charging", "b"),
    "LeftInEar": ("left_in_ear", "b"),
    "RightInEar": ("right_in_ear", "b"),
    "BothPodsInCase": ("both_pods_in_case", "b"),
    "CaseLidOpened": ("case_lid_opened", "b"),
    "FallbackBattery": ("fallback_battery", "i"),
}
SETTINGS = {"EarDetection": "ear_detection", "LidPopup": "lid_popup"}


def default_xml_path() -> str:
    here = os.path.dirname(os.path.abspath(__file__))
    return os.path.join(here, "..", "dbus", "org.pulsaros.AirPods.xml")


def snapshot_properties(snap: Snapshot) -> Dict[str, GLib.Variant]:
    return {
        name: GLib.Variant(sig, getattr(snap, attr)) for name, (attr, sig) in SNAPSHOT_PROPERTIES.items()
    }


class Service:
    def __init__(
        self,
        get_setting: Callable[[str], bool],
        set_setting: Callable[[str, bool], None],
        xml_path: str = "",
    ):
        with open(xml_path or default_xml_path(), encoding="utf-8") as f:
            self._info = Gio.DBusNodeInfo.new_for_xml(f.read()).interfaces[0]
        self._get_setting = get_setting
        self._set_setting = set_setting
        self._snapshot = Snapshot()
        self._conn = None
        self._reg_id = 0
        self._owner_id = 0

    def start(self) -> None:
        self._owner_id = Gio.bus_own_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameOwnerFlags.NONE,
            self._on_bus_acquired,
            None,
            self._on_name_lost,
        )

    def stop(self) -> None:
        if self._conn is not None and self._reg_id:
            self._conn.unregister_object(self._reg_id)
        if self._owner_id:
            Gio.bus_unown_name(self._owner_id)
        self._reg_id = 0
        self._owner_id = 0

    # -- publishing --------------------------------------------------------------------

    def publish(self, snapshot: Snapshot) -> None:
        old, self._snapshot = self._snapshot, snapshot
        old_props, new_props = snapshot_properties(old), snapshot_properties(snapshot)
        changed = {k: v for k, v in new_props.items() if v != old_props[k]}
        self._emit_changed(changed)

    def publish_setting(self, dbus_name: str) -> None:
        self._emit_changed({dbus_name: GLib.Variant("b", self._get_setting(SETTINGS[dbus_name]))})

    def emit_lid_opened(self) -> None:
        if self._conn is not None:
            self._conn.emit_signal(None, OBJECT_PATH, IFACE, "LidOpened", None)

    def _emit_changed(self, changed: Dict[str, GLib.Variant]) -> None:
        if not changed or self._conn is None:
            return
        self._conn.emit_signal(
            None,
            OBJECT_PATH,
            PROPERTIES_IFACE,
            "PropertiesChanged",
            GLib.Variant("(sa{sv}as)", (IFACE, changed, [])),
        )

    # -- GDBus callbacks ---------------------------------------------------------------

    def _on_bus_acquired(self, conn, name) -> None:
        self._conn = conn
        self._reg_id = conn.register_object(
            OBJECT_PATH, self._info, self._on_method_call, self._on_get_property, self._on_set_property
        )

    def _on_name_lost(self, conn, name) -> None:
        log.error("cannot own %s (is another instance running?)", name)

    def _on_method_call(self, conn, sender, path, iface, method, params, invocation) -> None:
        invocation.return_error_literal(
            Gio.dbus_error_quark(), Gio.DBusError.UNKNOWN_METHOD, f"no method {method}"
        )

    def _on_get_property(self, conn, sender, path, iface, name):
        if name in SNAPSHOT_PROPERTIES:
            attr, sig = SNAPSHOT_PROPERTIES[name]
            return GLib.Variant(sig, getattr(self._snapshot, attr))
        if name in SETTINGS:
            return GLib.Variant("b", self._get_setting(SETTINGS[name]))
        return None

    def _on_set_property(self, conn, sender, path, iface, name, value) -> bool:
        if name not in SETTINGS:
            return False
        self._set_setting(SETTINGS[name], bool(value.get_boolean()))
        self.publish_setting(name)
        return True
