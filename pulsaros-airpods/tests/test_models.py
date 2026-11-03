"""Seam 1 - domain: model id table, synced with AirPodsDesktop Source/Core/AppleCP.cpp GetModel.

Expected values are literals copied from upstream, not recomputed from the table under test.
"""

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(__file__))

from pulsaros_airpods import protocol  # noqa: E402
from pulsaros_airpods.protocol import Side  # noqa: E402
from test_protocol_state import make_packet  # noqa: E402

UPSTREAM = [
    (0x2002, "airpods_1", "AirPods 1"),
    (0x200F, "airpods_2", "AirPods 2"),
    (0x2013, "airpods_3", "AirPods 3"),
    (0x2019, "airpods_4", "AirPods 4"),
    (0x201B, "airpods_4_anc", "AirPods 4 (ANC)"),
    (0x2030, "airpods_5", "AirPods 5"),
    (0x2036, "airpods_5", "AirPods 5"),
    (0x200E, "airpods_pro", "AirPods Pro"),
    (0x2014, "airpods_pro_2", "AirPods Pro 2"),
    (0x2024, "airpods_pro_2_usb_c", "AirPods Pro 2 (USB-C)"),
    (0x2027, "airpods_pro_3", "AirPods Pro 3"),
    (0x200A, "airpods_max", "AirPods Max"),
    (0x201F, "airpods_max_usb_c", "AirPods Max (USB-C)"),
    (0x2012, "beats_fit_pro", "Beats Fit Pro"),
]


class ModelTableTests(unittest.TestCase):
    def test_every_upstream_id_maps_to_model_and_name(self):
        for model_id, slug, name in UPSTREAM:
            with self.subTest(model_id=hex(model_id)):
                model = protocol.model_from_id(model_id)
                self.assertEqual(model.value, slug)
                self.assertEqual(protocol.MODEL_NAMES[model], name)

    def test_packet_of_each_model_parses_to_that_model(self):
        for model_id, slug, _ in UPSTREAM:
            with self.subTest(model_id=hex(model_id)):
                adv = protocol.parse(make_packet(model_id, Side.LEFT))
                self.assertEqual(adv.model.value, slug)

    def test_unknown_ids_are_unknown(self):
        for model_id in (0x0000, 0x2003, 0x2005, 0x2006, 0xFFFF):  # 2003/5/6 are disabled upstream
            with self.subTest(model_id=hex(model_id)):
                self.assertEqual(protocol.model_from_id(model_id).value, "unknown")

    def test_only_max_models_are_headphones(self):
        self.assertEqual({m.value for m in protocol.HEADPHONE_MODELS},
                         {"airpods_max", "airpods_max_usb_c"})


if __name__ == "__main__":
    unittest.main()
