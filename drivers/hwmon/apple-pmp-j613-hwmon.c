// SPDX-License-Identifier: GPL-2.0-only OR MIT
/* Read-only J613/25G83 PMP telemetry. No cooling or firmware writes. */
#include <linux/hwmon.h>
#include <linux/io.h>
#include <linux/module.h>
#include <linux/of.h>
#include <linux/of_platform.h>
#include <linux/platform_device.h>
#include <linux/slab.h>
#include <linux/unaligned.h>
#include <linux/vmalloc.h>

#define SRAM_BASE 0x2d0500000ULL
#define SRAM_SIZE 0x80000
#define RECORD_SIZE 0xa4
#define NAME_OFFSET 0x94
#define CHANNELS 6

static const char * const names[CHANNELS] = {
	"ta000m", "te000m", "te001m", "tp000m", "tp001m", "tp002m",
};
static void __iomem *sram;
static u32 offsets[CHANNELS];
static struct device *hwmon;
static struct platform_device *parent;

/* Explicit aligned 32-bit accesses: never vector-copy unaligned SRAM MMIO. */
static void copy_record(u8 *out, u32 offset)
{
	unsigned int i;

	for (i = 0; i < RECORD_SIZE; i += 4)
		put_unaligned_le32(readl(sram + offset + i), out + i);
}

static int sample(unsigned int index, long *value)
{
	u8 a[RECORD_SIZE], b[RECORD_SIZE];
	s16 raw;
	unsigned int attempt;

	for (attempt = 0; attempt < 20; attempt++) {
		copy_record(a, offsets[index]);
		copy_record(b, offsets[index]);
		if (memcmp(a, b, sizeof(a)))
			continue;
		if (memcmp(a + NAME_OFFSET, names[index], 7))
			return -EIO;
		/* +0x30 is the current-valid byte. +0x48 is retained-valid. */
		if (a[0x30] != 1)
			return a[0x30] ? -EIO : -ENODATA;
		raw = (s16)get_unaligned_le16(a + 0x24);
		if (raw < -2560 || raw > 9600)
			return -ERANGE;
		*value = DIV_ROUND_CLOSEST((s64)raw * 1000, 64);
		return 0;
	}
	return -EAGAIN;
}

static umode_t visible(const void *data, enum hwmon_sensor_types type,
			      u32 attr, int channel)
{
	return type == hwmon_temp &&
	       (attr == hwmon_temp_input || attr == hwmon_temp_label) ? 0444 : 0;
}

static int read_temp(struct device *dev, enum hwmon_sensor_types type,
		     u32 attr, int channel, long *value)
{
	long hottest = LONG_MIN, reading;
	unsigned int i;
	int ret;

	if (type != hwmon_temp || attr != hwmon_temp_input)
		return -EOPNOTSUPP;
	if (channel < CHANNELS)
		return sample(channel, value);
	for (i = 0; i < CHANNELS; i++) {
		ret = sample(i, &reading);
		if (ret == -ENODATA)
			continue;
		if (ret)
			return ret;
		hottest = max(hottest, reading);
	}
	if (hottest == LONG_MIN)
		return -ENODATA;
	*value = hottest;
	return 0;
}

static int read_label(struct device *dev, enum hwmon_sensor_types type,
		      u32 attr, int channel, const char **value)
{
	if (type != hwmon_temp || attr != hwmon_temp_label)
		return -EOPNOTSUPP;
	*value = channel < CHANNELS ? names[channel] : "PMP valid-sensor maximum";
	return 0;
}

static const struct hwmon_ops ops = {
	.is_visible = visible,
	.read = read_temp,
	.read_string = read_label,
};
static const struct hwmon_channel_info * const channels[] = {
	HWMON_CHANNEL_INFO(temp,
		HWMON_T_INPUT | HWMON_T_LABEL, HWMON_T_INPUT | HWMON_T_LABEL,
		HWMON_T_INPUT | HWMON_T_LABEL, HWMON_T_INPUT | HWMON_T_LABEL,
		HWMON_T_INPUT | HWMON_T_LABEL, HWMON_T_INPUT | HWMON_T_LABEL,
		HWMON_T_INPUT | HWMON_T_LABEL),
	NULL,
};
static const struct hwmon_chip_info chip = { .ops = &ops, .info = channels };

static int __init pmp_j613_init(void)
{
	struct device_node *node;
	const char *uuid;
	u8 *snapshot;
	unsigned int i, j, hits;
	int ret;

	if (!of_machine_is_compatible("apple,j613"))
		return -ENODEV;
	node = of_find_node_by_path("/soc/pmp@2d0500000");
	if (!node)
		return -ENODEV;
	ret = of_property_read_string(node, "apple,tunable-uuid", &uuid);
	if (ret || strcmp(uuid, "EFD60284-7C58-33A9-8522-CB3366AF6040") ||
	    !of_device_is_compatible(node, "apple,t8122-pmp-v2")) {
		of_node_put(node);
		return -ENODEV;
	}
	parent = of_find_device_by_node(node);
	of_node_put(node);
	if (!parent)
		return -ENODEV;
	sram = ioremap(SRAM_BASE, SRAM_SIZE);
	if (!sram) {
		ret = -ENOMEM;
		goto put_parent;
	}
	snapshot = vmalloc(SRAM_SIZE);
	if (!snapshot) {
		ret = -ENOMEM;
		goto unmap;
	}
	for (i = 0; i < SRAM_SIZE; i += 4)
		put_unaligned_le32(readl(sram + i), snapshot + i);
	for (j = 0; j < CHANNELS; j++) {
		hits = 0;
		for (i = NAME_OFFSET; i + 16 <= SRAM_SIZE; i += 4) {
			if (!memcmp(snapshot + i, names[j], 7) &&
			    get_unaligned_le32(snapshot + i - NAME_OFFSET) == j + 1 &&
			    get_unaligned_le32(snapshot + i - NAME_OFFSET + 4) == 6) {
				offsets[j] = i - NAME_OFFSET;
				hits++;
			}
		}
		if (hits != 1 || offsets[j] + RECORD_SIZE > SRAM_SIZE ||
		    (j && offsets[j] != offsets[0] + j * RECORD_SIZE) ||
		    get_unaligned_le32(snapshot + offsets[j]) != j + 1 ||
		    get_unaligned_le32(snapshot + offsets[j] + 4) != 6) {
			ret = -EIO;
			goto free_snapshot;
		}
	}
	hwmon = hwmon_device_register_with_info(&parent->dev, "j613_pmp", NULL, &chip, NULL);
	ret = PTR_ERR_OR_ZERO(hwmon);
	if (!ret)
		pr_info("j613-pmp: read-only six-channel telemetry; current samples only\n");
free_snapshot:
	vfree(snapshot);
	if (!ret)
		return 0;
unmap:
	iounmap(sram);
put_parent:
	put_device(&parent->dev);
	return ret;
}

static void __exit pmp_j613_exit(void)
{
	hwmon_device_unregister(hwmon);
	iounmap(sram);
	put_device(&parent->dev);
}

/*
 * There is no driver to bind: the PMP driver owns the device. The table only
 * gives udev a modalias, so the module loads once the PMP device appears
 * (on 25G83, appledrm enables it); init then applies the J613 checks above.
 */
static const struct of_device_id pmp_j613_of_match[] = {
	{ .compatible = "apple,t8122-pmp-v2" },
	{}
};
MODULE_DEVICE_TABLE(of, pmp_j613_of_match);

module_init(pmp_j613_init);
module_exit(pmp_j613_exit);
MODULE_LICENSE("Dual MIT/GPL");
MODULE_DESCRIPTION("Read-only J613 25G83 PMP current temperature telemetry");
