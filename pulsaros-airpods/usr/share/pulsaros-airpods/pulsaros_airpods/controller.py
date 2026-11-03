"""Glue between the connected device (classic BT) and BLE advertisements.

Pure logic (no D-Bus, no GLib) so it is unit-testable. The daemon owns the I/O.
"""

import logging
from dataclasses import dataclass
from typing import Callable, Optional

from . import protocol
from .protocol import HEADPHONE_MODELS, MODEL_NAMES, Model
from .state import State, StateManager
from .tracker import DeviceInfo

log = logging.getLogger(__name__)


@dataclass(frozen=True)
class Snapshot:
    """What is published on D-Bus. -1 / empty / False mean unavailable (see the XML)."""

    connected: bool = False
    name: str = ""
    address: str = ""
    model_id: str = "unknown"
    model: str = ""
    is_headphones: bool = False
    has_advertisement: bool = False
    left: int = -1
    right: int = -1
    case: int = -1
    left_charging: bool = False
    right_charging: bool = False
    case_charging: bool = False
    left_in_ear: bool = False
    right_in_ear: bool = False
    both_pods_in_case: bool = False
    case_lid_opened: bool = False
    fallback_battery: int = -1


def _pct(value: Optional[int]) -> int:
    return -1 if value is None else value


def build_snapshot(device: Optional[DeviceInfo], state: Optional[State]) -> Snapshot:
    if device is None:
        return Snapshot()

    model = device.model
    if state is not None and state.model is not Model.UNKNOWN:
        model = state.model

    base = dict(
        connected=True,
        name=device.name,
        address=device.address,
        model_id=model.value,
        model=MODEL_NAMES[model] if model is not Model.UNKNOWN else "",
        is_headphones=model in HEADPHONE_MODELS,
        fallback_battery=_pct(device.battery),
    )
    if state is None:
        return Snapshot(**base)

    return Snapshot(
        **base,
        has_advertisement=True,
        left=_pct(state.left.battery),
        right=_pct(state.right.battery),
        case=_pct(state.case.battery),
        left_charging=state.left.charging,
        right_charging=state.right.charging,
        case_charging=state.case.charging,
        left_in_ear=state.left.in_ear,
        right_in_ear=state.right.in_ear,
        both_pods_in_case=state.case.both_pods_in_case,
        case_lid_opened=state.case.lid_opened,
    )


def _both_in_ear(state: Optional[State]) -> bool:
    return state is not None and state.left.in_ear and state.right.in_ear


class Controller:
    def __init__(
        self,
        state_manager: StateManager,
        on_snapshot: Callable[[Snapshot], None],
        on_lid_opened: Callable[[], None] = lambda: None,
        on_both_in_ear: Callable[[bool], None] = lambda _: None,
    ):
        self._mgr = state_manager
        self._on_snapshot = on_snapshot
        self._on_lid_opened = on_lid_opened
        self._on_both_in_ear = on_both_in_ear
        self._device: Optional[DeviceInfo] = None
        self._snapshot = Snapshot()

    @property
    def snapshot(self) -> Snapshot:
        return self._snapshot

    def set_device(self, device: Optional[DeviceInfo]) -> None:
        """Called whenever the tracker's active device may have changed."""
        old = self._device
        self._device = device
        if device is None or old is None or old.address != device.address:
            event = self._mgr.disconnect()
            if event is not None and _both_in_ear(event.old_state):
                self._on_both_in_ear(False)
        self._publish()

    def on_adv(self, address: str, rssi: int, data: bytes) -> None:
        if self._device is None:
            return  # AirPods are not connected: ignore every advertisement

        adv = protocol.parse(data)
        if adv is None:
            return
        expected = self._device.model
        if (
            expected is not Model.UNKNOWN
            and adv.model is not Model.UNKNOWN
            and adv.model is not expected
        ):
            return  # someone else's AirPods

        event = self._mgr.on_adv(address, rssi, adv)
        if event is None:
            return

        old, new = event.old_state, event.new_state
        if new.case.lid_opened and not (old is not None and old.case.lid_opened):
            self._on_lid_opened()
        if _both_in_ear(old) != _both_in_ear(new):
            self._on_both_in_ear(_both_in_ear(new))
        self._publish()

    def tick(self) -> None:
        """Run the lost/reset timers. Call about once a second."""
        event = self._mgr.expire()
        if event is not None:
            if _both_in_ear(event.old_state):
                self._on_both_in_ear(False)
            self._publish()

    def refresh(self) -> None:
        """Re-publish, e.g. after the BlueZ Battery1 level changed."""
        self._publish()

    def _publish(self) -> None:
        snap = build_snapshot(self._device, self._mgr.state if self._device else None)
        if snap != self._snapshot:
            self._snapshot = snap
            self._on_snapshot(snap)
