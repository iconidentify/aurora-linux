/* SPDX-License-Identifier: GPL-2.0-only OR MIT */
#ifndef APPLE_J613_25G83_H
#define APPLE_J613_25G83_H

#include <linux/string.h>
#include <linux/types.h>

#define J613_25G83_DCP_UUID "C042E95C-B9D8-3F0E-94B3-582A08AA6FDD"
#define J613_25G83_PMP_UUID "EFD60284-7C58-33A9-8522-CB3366AF6040"
#define J613_25G83_HANDOFF "apple,j613-25g83-mapping-handoff"
/*
 * The J615 runs the same 25G83 DCP, PMP and GPU images (one restore image,
 * shared firmware entries), so the "j613-25g83" names are the firmware
 * profile. It is admitted only experimentally: the boot loader must also have
 * /chosen/asahi,j615-25g83-experimental = "1" (m1n1.conf), which it requires
 * before it hands a J615's display or GPU over.
 */
#define J615_25G83_OPT_IN "asahi,j615-25g83-experimental"

/* The board half of the identity: a J613, or a J615 with the opt-in. */
static inline bool t8122_25g83_board(bool j613, bool j615, const char *optin,
				     int optin_len)
{
	return j613 || (j615 && optin && optin_len == 2 && optin[0] == '1' &&
			!optin[1]);
}

/* `board`: t8122_25g83_board() for the running machine. */
static inline bool j613_25g83_identity(bool board, u32 profile,
				     const u32 *version, size_t cells,
				     const char *uuid)
{
	return board && profile == 1 && version && cells == 3 &&
	       version[0] == 26 && version[1] == 6 && version[2] == 2 &&
	       uuid && !strcmp(uuid, J613_25G83_DCP_UUID);
}

static inline bool j613_25g83_clock(const u32 *witness, size_t cells)
{
	return witness && cells == 5 && witness[0] == 1 && witness[1] == 345 &&
	       witness[2] == 416 && witness[3] && !witness[4];
}

static inline bool j613_25g83_mappings(const u32 *markers, size_t cells)
{
	return markers && cells == 3 && markers[0] == 1 && markers[1] == 1 &&
	       markers[2] == 1;
}

#endif
