/* cc-hub-header.h
 *
 * Hub card shown at the top of a panel: squircle icon, bold title, gray
 * description and an optional "Learn more" link. No toggle.
 *
 * SPDX-License-Identifier: GPL-2.0-or-later
 */

#pragma once

#include <adwaita.h>

G_BEGIN_DECLS

#define CC_TYPE_HUB_HEADER (cc_hub_header_get_type ())
G_DECLARE_FINAL_TYPE (CcHubHeader, cc_hub_header, CC, HUB_HEADER, AdwBin)

G_END_DECLS
