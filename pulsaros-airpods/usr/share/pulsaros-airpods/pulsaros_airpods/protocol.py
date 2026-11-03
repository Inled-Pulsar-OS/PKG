"""Apple Continuity Protocol "Proximity Pairing" packet parser.

Pure Python, no I/O and no D-Bus. Ported from AirPodsDesktop (Source/Core/AppleCP.*,
AirPodsAdvertisement.cpp).

Packet layout (27 bytes, little endian):

    0      packet type (0x07 = ProximityPairing)
    1      remaining length (25)
    2      unknown
    3-4    model id (uint16)
    5      flags: bit1 curr in ear, bit2 both in case, bit3 another in ear,
                  bit5 broadcast from left
    6      battery: low nibble = curr, high nibble = another   (0..10, else unavailable)
    7      low nibble = case battery (0..10), bit4 curr charging, bit5 another charging,
           bit6 case charging
    8      lid: bits0-2 switch count, bit3 closed
    9      color
    10     unknown
    11-26  hash / encrypted payload (never logged, may identify the user)

The packet has no left/right fields, only "current" (the earbud that broadcasts) and
"another". The broadcast-from bit says which one is which.
"""

from dataclasses import dataclass
from enum import Enum
from typing import Optional

VENDOR_ID = 76
PACKET_LENGTH = 27
PACKET_TYPE_PROXIMITY_PAIRING = 0x07
REMAINING_LENGTH = PACKET_LENGTH - 2


class Model(str, Enum):
    UNKNOWN = "unknown"
    AIRPODS_1 = "airpods_1"
    AIRPODS_2 = "airpods_2"
    AIRPODS_3 = "airpods_3"
    AIRPODS_4 = "airpods_4"
    AIRPODS_4_ANC = "airpods_4_anc"
    AIRPODS_5 = "airpods_5"
    AIRPODS_PRO = "airpods_pro"
    AIRPODS_PRO_2 = "airpods_pro_2"
    AIRPODS_PRO_2_USB_C = "airpods_pro_2_usb_c"
    AIRPODS_PRO_3 = "airpods_pro_3"
    AIRPODS_MAX = "airpods_max"
    AIRPODS_MAX_USB_C = "airpods_max_usb_c"
    BEATS_FIT_PRO = "beats_fit_pro"


MODEL_NAMES = {
    Model.UNKNOWN: "Unknown",
    Model.AIRPODS_1: "AirPods 1",
    Model.AIRPODS_2: "AirPods 2",
    Model.AIRPODS_3: "AirPods 3",
    Model.AIRPODS_4: "AirPods 4",
    Model.AIRPODS_4_ANC: "AirPods 4 (ANC)",
    Model.AIRPODS_5: "AirPods 5",
    Model.AIRPODS_PRO: "AirPods Pro",
    Model.AIRPODS_PRO_2: "AirPods Pro 2",
    Model.AIRPODS_PRO_2_USB_C: "AirPods Pro 2 (USB-C)",
    Model.AIRPODS_PRO_3: "AirPods Pro 3",
    Model.AIRPODS_MAX: "AirPods Max",
    Model.AIRPODS_MAX_USB_C: "AirPods Max (USB-C)",
    Model.BEATS_FIT_PRO: "Beats Fit Pro",
}

_MODEL_IDS = {
    0x2002: Model.AIRPODS_1,
    0x200F: Model.AIRPODS_2,
    0x2013: Model.AIRPODS_3,
    0x2019: Model.AIRPODS_4,
    0x201B: Model.AIRPODS_4_ANC,
    0x2030: Model.AIRPODS_5,
    0x2036: Model.AIRPODS_5,
    0x200E: Model.AIRPODS_PRO,
    0x2014: Model.AIRPODS_PRO_2,
    0x2024: Model.AIRPODS_PRO_2_USB_C,
    0x2027: Model.AIRPODS_PRO_3,
    0x200A: Model.AIRPODS_MAX,
    0x201F: Model.AIRPODS_MAX_USB_C,
    0x2012: Model.BEATS_FIT_PRO,
}

# Models without their own artwork: no case, so case fields are meaningless.
HEADPHONE_MODELS = frozenset({Model.AIRPODS_MAX, Model.AIRPODS_MAX_USB_C})


def model_from_id(model_id: int) -> Model:
    return _MODEL_IDS.get(model_id, Model.UNKNOWN)


class Side(str, Enum):
    LEFT = "left"
    RIGHT = "right"


@dataclass(frozen=True)
class PodState:
    battery: Optional[int] = None  # percent, None = unavailable
    charging: bool = False
    in_ear: bool = False


@dataclass(frozen=True)
class CaseState:
    battery: Optional[int] = None
    charging: bool = False
    both_pods_in_case: bool = False
    lid_opened: bool = False


@dataclass(frozen=True)
class AdvState:
    """State decoded from one advertisement."""

    model: Model
    side: Side
    left: PodState
    right: PodState
    case: CaseState


def is_valid(data: bytes) -> bool:
    return (
        len(data) == PACKET_LENGTH
        and data[0] == PACKET_TYPE_PROXIMITY_PAIRING
        and data[1] == REMAINING_LENGTH
    )


def _battery(nibble: int) -> Optional[int]:
    return nibble * 10 if 0 <= nibble <= 10 else None


def parse(data: bytes) -> Optional[AdvState]:
    """Decode a ProximityPairing packet. Returns None if the packet is not valid."""
    data = bytes(data)
    if not is_valid(data):
        return None

    model = model_from_id(data[3] | (data[4] << 8))

    flags = data[5]
    curr_in_ear = bool(flags & (1 << 1))
    both_in_case = bool(flags & (1 << 2))
    anot_in_ear = bool(flags & (1 << 3))
    from_left = bool(flags & (1 << 5))
    side = Side.LEFT if from_left else Side.RIGHT

    curr_battery = _battery(data[6] & 0x0F)
    anot_battery = _battery(data[6] >> 4)
    case_battery = _battery(data[7] & 0x0F)
    curr_charging = bool(data[7] & (1 << 4))
    anot_charging = bool(data[7] & (1 << 5))
    case_charging = bool(data[7] & (1 << 6))

    lid_closed = bool(data[8] & (1 << 3))

    # "Flipped": the earbud that broadcasts is "curr", the other one is "anot".
    if from_left:
        left_battery, right_battery = curr_battery, anot_battery
        left_charging, right_charging = curr_charging, anot_charging
        left_ear_raw, right_ear_raw = curr_in_ear, anot_in_ear
    else:
        left_battery, right_battery = anot_battery, curr_battery
        left_charging, right_charging = anot_charging, curr_charging
        left_ear_raw, right_ear_raw = anot_in_ear, curr_in_ear

    # While charging, the "ear" bit can be set on one of the devices; filter it out.
    left = PodState(left_battery, left_charging, (not left_charging) and left_ear_raw)
    right = PodState(right_battery, right_charging, (not right_charging) and right_ear_raw)
    case = CaseState(case_battery, case_charging, both_in_case, not lid_closed)

    return AdvState(model, side, left, right, case)
