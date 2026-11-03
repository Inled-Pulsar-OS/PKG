"""Tracks which BlueZ device is "our" AirPods (classic Bluetooth connection).

AirPods advertise with random BLE addresses, so the BLE side cannot identify a device.
The classic connection does: BlueZ Device1 has a stable Address, and its Modalias carries
Apple's vendor id and the model id (the same 0x20xx id as in the BLE packet).
"""

import re
from dataclasses import dataclass
from typing import Callable, Dict, Optional

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
        on_bind: Optional[Callable[[str], None]] = None,
    ):
        self._bound = bound_address
        self._on_bind = on_bind
        self._devices: Dict[str, dict] = {}  # path -> merged props
        self._batteries: Dict[str, int] = {}

    def update(self, path: str, props: dict) -> None:
        merged = self._devices.setdefault(path, {})
        merged.update(props)

    def update_battery(self, path: str, percentage: Optional[int]) -> None:
        if percentage is None:
            self._batteries.pop(path, None)
        else:
            self._batteries[path] = int(percentage)

    def remove(self, path: str) -> None:
        self._devices.pop(path, None)
        self._batteries.pop(path, None)

    def clear(self) -> None:
        self._devices.clear()
        self._batteries.clear()

    @property
    def active(self) -> Optional[DeviceInfo]:
        """The bound device if connected, else the first connected AirPods (and bind it)."""
        connected = [
            (p, d)
            for p, d in self._devices.items()
            if d.get("Connected") and is_airpods(d) and d.get("Address")
        ]
        if not connected:
            return None

        chosen = None
        if self._bound:
            chosen = next((pd for pd in connected if str(pd[1]["Address"]) == self._bound), None)
            if chosen is None:
                return None  # a bound pair exists but another pair is connected: ignore it
        else:
            chosen = connected[0]
            self._bound = str(chosen[1]["Address"])
            if self._on_bind:
                self._on_bind(self._bound)

        path, d = chosen
        return DeviceInfo(
            address=str(d["Address"]),
            name=str(d.get("Alias") or d.get("Name") or ""),
            model=model_of(d),
            battery=self._batteries.get(path),
        )
