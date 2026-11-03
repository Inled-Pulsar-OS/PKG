"""Low-battery alerts with macOS-like thresholds. Pure logic, no I/O."""

from typing import Dict, List, Optional, Tuple

THRESHOLDS = (5, 10, 20)  # percent, ascending


def _bucket(level: int) -> Optional[int]:
    """Lowest threshold the level is at or under, or None when the level is fine."""
    return next((t for t in THRESHOLDS if level <= t), None)


class BatteryAlerts:
    """Turns a stream of Snapshots into (which, level) alerts.

    One alert per threshold crossing. A level that stays low alerts once; a level that
    rises above a threshold (or starts charging, or disconnects) re-arms it.
    """

    def __init__(self) -> None:
        self._notified: Dict[str, Optional[int]] = {}

    def update(self, snapshot) -> List[Tuple[str, int]]:
        if not snapshot.connected or not snapshot.has_advertisement:
            self._notified.clear()
            return []

        if snapshot.is_headphones:
            level = snapshot.left if snapshot.left >= 0 else snapshot.right
            readings = {"headphones": (level, snapshot.left_charging)}
        else:
            readings = {
                "left": (snapshot.left, snapshot.left_charging),
                "right": (snapshot.right, snapshot.right_charging),
                "case": (snapshot.case, snapshot.case_charging),
            }

        alerts = []
        for which, (level, charging) in readings.items():
            if charging:
                self._notified[which] = None
            elif level >= 0 and self._crossed(which, level):
                alerts.append((which, level))
        return alerts

    def _crossed(self, which: str, level: int) -> bool:
        bucket = _bucket(level)
        notified = self._notified.get(which)
        self._notified[which] = bucket
        return bucket is not None and (notified is None or bucket < notified)
