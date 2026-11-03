"""Tracks which BlueZ device is "our" AirPods (classic Bluetooth connection).

AirPods advertise with random BLE addresses, so the BLE side cannot identify a device.
The classic connection does: BlueZ Device1 has a stable Address, and its Modalias carries
Apple's vendor id and the model id (the same 0x20xx id as in the BLE packet).
"""

import re
from dataclasses import dataclass
from typing import Callable, Dict, List, Optional, Set, Tuple

from .protocol import Model, model_from_id

_MODALIAS = re.compile(r"^(bluetooth|usb):v([0-9A-Fa-f]{4})p([0-9A-Fa-f]{4})")
_NAME = re.compile(r"airpods|beats\s*(fit|studio|solo|flex|x|powerbeats)?", re.IGNORECASE)
_APPLE_BT_VENDOR = 0x004C
_APPLE_USB_VENDOR = 0x05AC


@dataclass(frozen=True)
class DeviceInfo:
    address: str
    name: str
    model: Model
    battery: Optional[int]  # BlueZ Battery1 percentage, None if absent


def parse_modalias(modalias: str):
    """Return (vendor, product) integers or None."""
    m = _MODALIAS.match(modalias or "")
    if not m:
        return None
    return int(m.group(2), 16), int(m.group(3), 16)


def is_airpods(props: dict) -> bool:
    """Heuristic: an Apple audio device, or a name that says so."""
    parsed = parse_modalias(str(props.get("Modalias", "")))
    if parsed is not None:
        vendor, product = parsed
        scheme_bt = str(props.get("Modalias", "")).startswith("bluetooth:")
        apple = vendor == (_APPLE_BT_VENDOR if scheme_bt else _APPLE_USB_VENDOR)
        audio = str(props.get("Icon", "")).startswith("audio-")
        if apple and (model_from_id(product) is not Model.UNKNOWN or audio):
            return True
    name = str(props.get("Alias") or props.get("Name") or "")
    return bool(_NAME.search(name)) and str(props.get("Icon", "audio-")).startswith("audio-")


def model_of(props: dict) -> Model:
    parsed = parse_modalias(str(props.get("Modalias", "")))
    return model_from_id(parsed[1]) if parsed else Model.UNKNOWN


class Tracker:
    """Feed it BlueZ Device1 property dicts; read the active connected AirPods."""

    def __init__(
        self,
        bound_address: Optional[str] = None,
        on_bind: Optional[Callable[[Optional[str]], None]] = None,
    ):
        self._bound = bound_address  # the last active pair; None after forget()
        self._on_bind = on_bind
        self._devices: Dict[str, dict] = {}  # path -> merged props
        self._batteries: Dict[str, int] = {}
        self._connected_at: Dict[str, int] = {}  # path -> order of the latest connection
        self._counter = 0
        self._forgotten: Set[str] = set()  # addresses hidden until they disconnect

    def update(self, path: str, props: dict) -> None:
        merged = self._devices.setdefault(path, {})
        was_connected = bool(merged.get("Connected"))
        merged.update(props)
        if merged.get("Connected"):
            if not was_connected:
                self._counter += 1
                self._connected_at[path] = self._counter
        elif was_connected:
            self._disconnected(path, merged)

    def _disconnected(self, path: str, device: dict) -> None:
        self._connected_at.pop(path, None)
        self._forgotten.discard(str(device.get("Address", "")))

    def update_battery(self, path: str, percentage: Optional[int]) -> None:
        if percentage is None:
            self._batteries.pop(path, None)
        else:
            self._batteries[path] = int(percentage)

    def remove(self, path: str) -> None:
        device = self._devices.pop(path, None)
        if device is not None:
            self._disconnected(path, device)
        self._batteries.pop(path, None)

    def clear(self) -> None:
        self._devices.clear()
        self._batteries.clear()
        self._connected_at.clear()
        self._forgotten.clear()

    def forget(self) -> None:
        """Stop showing the active pair until it disconnects and connects again."""
        device = self.active
        if device is None:
            return
        self._forgotten.add(device.address)
        self._bind(None)

    def paired(self) -> List[Tuple[str, str, bool]]:
        """Every paired (or connected) AirPods as (address, name, connected), sorted by name."""
        rows = [
            (str(d["Address"]), str(d.get("Alias") or d.get("Name") or ""), bool(d.get("Connected")))
            for d in self._devices.values()
            if d.get("Address") and is_airpods(d) and (d.get("Paired") or d.get("Connected"))
        ]
        return sorted(rows, key=lambda r: (r[1].lower(), r[0]))

    def _bind(self, address: Optional[str]) -> None:
        if address != self._bound:
            self._bound = address
            if self._on_bind:
                self._on_bind(address)

    @property
    def active(self) -> Optional[DeviceInfo]:
        """The most recently connected AirPods. A change of pair is persisted as the binding."""
        connected = [
            (p, d)
            for p, d in self._devices.items()
            if d.get("Connected")
            and is_airpods(d)
            and d.get("Address")
            and str(d["Address"]) not in self._forgotten
        ]
        if not connected:
            return None

        path, d = max(connected, key=lambda pd: self._connected_at.get(pd[0], 0))
        self._bind(str(d["Address"]))
        return DeviceInfo(
            address=str(d["Address"]),
            name=str(d.get("Alias") or d.get("Name") or ""),
            model=model_of(d),
            battery=self._batteries.get(path),
        )
