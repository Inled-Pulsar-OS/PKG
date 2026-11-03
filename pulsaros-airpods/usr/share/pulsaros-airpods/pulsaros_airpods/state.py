"""Merges advertisements from the two earbuds into one device state.

Pure logic, no threads: time is injected so the lost/reset timers can be tested.
Ported from AirPodsDesktop (Source/Core/AirPodsStateManager.cpp).

AirPods use random non-resolvable BLE addresses, so a device cannot be identified by
address. This module tracks "our" device heuristically (model, battery drift, RSSI drift).
"""

import time
from dataclasses import dataclass
from typing import Callable, Optional

from .protocol import AdvState, CaseState, Model, PodState, Side

LOST_TIMEOUT = 10.0  # seconds without any advertisement: the device is lost
SIDE_RESET_TIMEOUT = 10.0  # seconds without an advertisement from one side: forget that side
MAX_BATTERY_DIFF = 10  # percent steps are multiples of 10: more than one step is suspicious
MAX_RSSI_DIFF = 50
DEFAULT_RSSI_MIN = -80


@dataclass(frozen=True)
class State:
    model: Model = Model.UNKNOWN
    left: PodState = PodState()
    right: PodState = PodState()
    case: CaseState = CaseState()


@dataclass(frozen=True)
class UpdateEvent:
    old_state: Optional[State]
    new_state: State


@dataclass
class _Entry:
    address: str
    rssi: int
    adv: AdvState
    stamp: float


def _battery_diff(a: Optional[int], b: Optional[int]) -> int:
    if a is None or b is None:
        return 0
    return abs(a - b)


class StateManager:
    def __init__(
        self,
        rssi_min: int = DEFAULT_RSSI_MIN,
        clock: Callable[[], float] = time.monotonic,
    ):
        self._clock = clock
        self._rssi_min = rssi_min
        self._entries = {Side.LEFT: None, Side.RIGHT: None}
        self._state: Optional[State] = None
        self._last_adv: float = 0.0

    # -- public ------------------------------------------------------------------------

    @property
    def state(self) -> Optional[State]:
        return self._state

    def set_rssi_min(self, rssi_min: int) -> None:
        self._rssi_min = rssi_min

    def on_adv(self, address: str, rssi: int, adv: AdvState) -> Optional[UpdateEvent]:
        """Feed one advertisement. Returns an event only if the merged state changed."""
        if not self._is_possible_desired(address, rssi, adv):
            return None

        now = self._clock()
        self._last_adv = now
        self._entries[adv.side] = _Entry(address, rssi, adv, now)
        return self._update_state()

    def expire(self) -> Optional[UpdateEvent]:
        """Apply the timers. Call about once a second.

        Returns an event with ``new_state`` equal to the old state only when the device
        was lost; use ``state is None`` afterwards to detect that.
        """
        now = self._clock()
        if self._state is not None and now - self._last_adv >= LOST_TIMEOUT:
            return self._reset_all()

        # A stale side is forgotten, but the merged state is kept until the whole device
        # is lost, like the original.
        for side, entry in self._entries.items():
            if entry is not None and now - entry.stamp >= SIDE_RESET_TIMEOUT:
                self._entries[side] = None
        return None

    def disconnect(self) -> Optional[UpdateEvent]:
        return self._reset_all()

    # -- internals ---------------------------------------------------------------------

    def _reset_all(self) -> Optional[UpdateEvent]:
        old = self._state
        self._entries = {Side.LEFT: None, Side.RIGHT: None}
        self._state = None
        if old is None:
            return None
        return UpdateEvent(old, old)

    def _is_possible_desired(self, address: str, rssi: int, adv: AdvState) -> bool:
        if rssi < self._rssi_min:
            return False

        other_side = Side.RIGHT if adv.side is Side.LEFT else Side.LEFT
        last = self._entries[adv.side]
        last_other = self._entries[other_side]

        for cached in (last, last_other):
            if (
                cached is not None
                and cached.adv.model is not Model.UNKNOWN
                and adv.model is not Model.UNKNOWN
                and cached.adv.model is not adv.model
            ):
                return False

        if last is not None and last.address != address:
            if (
                _battery_diff(adv.left.battery, last.adv.left.battery) > MAX_BATTERY_DIFF
                or _battery_diff(adv.right.battery, last.adv.right.battery) > MAX_BATTERY_DIFF
                or _battery_diff(adv.case.battery, last.adv.case.battery) > MAX_BATTERY_DIFF
            ):
                return False
            if abs(rssi - last.rssi) > MAX_RSSI_DIFF:
                return False

        if last_other is not None and abs(rssi - last_other.rssi) > MAX_RSSI_DIFF:
            return False

        return True

    def _update_state(self) -> Optional[UpdateEvent]:
        left, right = self._entries[Side.LEFT], self._entries[Side.RIGHT]

        def pick(available) -> Optional[AdvState]:
            """Pick the entry that has the field; the most recent one if both do."""
            a = left is not None and available(left.adv)
            b = right is not None and available(right.adv)
            if a and b:
                return left.adv if left.stamp > right.stamp else right.adv
            if a:
                return left.adv
            if b:
                return right.adv
            return None

        # Original falls back to the right entry when neither has the field; keep that.
        fallback = left.adv if left is not None and right is None else (
            right.adv if right is not None else left.adv
        )

        model_src = pick(lambda s: s.model is not Model.UNKNOWN) or fallback
        left_src = pick(lambda s: s.left.battery is not None) or fallback
        right_src = pick(lambda s: s.right.battery is not None) or fallback
        case_src = pick(lambda s: s.case.battery is not None) or fallback

        new = State(model_src.model, left_src.left, right_src.right, case_src.case)
        if new == self._state:
            return None
        old, self._state = self._state, new
        return UpdateEvent(old, new)
