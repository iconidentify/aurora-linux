# Thunderbolt PCIe (PCIe-C) test report

On M1 Pro/Max (T600x) and M2 Pro/Max (T602x) Macs, PCIe over
Thunderbolt is off unless the kernel is booted with
`pcie_apple.tunnel_kernel_init=1`. With it, the PCIe devices inside
Thunderbolt 3 docks and displays come up: for an Apple Studio Display, its
USB-C ports, camera, speakers and microphone.

To test it:

1. Add `pcie_apple.tunnel_kernel_init=1` to the kernel command line and
   reboot. In GRUB, press `e` on the boot entry to add it for one boot.
2. Plug in a Thunderbolt 3 dock or display. USB-C hubs and DisplayPort-only
   monitors do not use the PCIe tunnel and do not test it.
3. Run `tools/aurora-tb/tb-pcie-report` (with `sudo` if your user cannot
   read the kernel log) and post the report it saves.

Worth adding to the report: whether the device still works after
unplugging and replugging it, after power-cycling it, and after
`systemctl suspend`.

A tunnel whose DART has no tunables in the device tree stays off. The
report then prints a read-only `ioreg` command to run in macOS on the same
Mac; its output lets those values be added.
