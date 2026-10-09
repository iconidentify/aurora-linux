# M3 GPU quick start

For the **13-inch and 15-inch M3 MacBook Air (J613/J615)** with the supported
14.8.3 stub, the matched installer prepares the experimental profile.

**Current status:** current14 firmware startup still stalls on reported J613
installs ([#35](https://github.com/iconidentify/aurora-linux/issues/35)), even
after correcting PMP power admission. Installation and a working panel do
not establish acceleration. Native25 remains a separate J613 GL-only route
requiring its own exact firmware and matched stage1; migration instructions
are pending.

1. Enable the experimental GPU profile:

   ```sh
   curl -fsSL https://github.com/iconidentify/aurora-linux/releases/latest/download/install-aurora-sep.sh | bash -s -- --m3-gpu
   ```

2. Reboot and log into your desktop normally.

3. Run:

   ```sh
   aurora-m3-gpu-check
   ```

Successful acceleration must report **PASS Apple GPU OpenGL (red/blue readback)** and, on the legacy
profile, **PASS Apple GPU Vulkan (compute readback)**. This runs real GPU
work through the desktop session and checks its current Mesa profile,
render node and loaded libraries. An installed package or working panel
alone does not prove acceleration. Use `--details` only when reporting a
failure. The check does not qualify compositor scanout or GPU conformance.

If boot or login fails, select **Aurora previous (GPU off)** in Limine or
the retained previous kernel in GRUB; report the exact error with the
same installer's `--m3-report` output.

The matched installer keeps a GPU-off fallback. `--m3-gpu-experiment`
only installs one-shot tools and is unnecessary for this persistent route.
The command and installed checker are available in **Aurora 2026.10.09**;
12.6 supports Air 13 with `--m3-gpu-persistent` and its Mesa probe instead.

| Mac | Route |
| --- | --- |
| Air 13-inch J613 / Air 15-inch J615 | `--m3-gpu` with the new matched bundle |
| Pro 16-inch J516S | Default supported display/GPU handoff install |
| Pro 14-inch J514S | First activate with `--m3-handoff` |
| Base-M3 Pro/iMac, M3 Max | GPU handoff support remains pending |

An existing Pro desktop can run the downloaded installer with
`--m3-gpu-check` without installing or changing packages.

## Firmware profiles

System firmware 26.6.2 does **not** select GPU firmware. The legacy
OS-firmware label can be 14.7 while its actual stub and GPU image are
14.8.3. The installer reads exact GPU compatibility when available and
checks the actual stub/iBoot when an unarmed legacy boot omits it. Clean
`v1.6.1` stage 1 is admitted only under the matched legacy Air contract,
with the exact stub/iBoot and no log overlap. It does not flash stage 1.

J613 already booted from its own exact 26.6.2/25G83 volume group with an
existing matched `v1.6.1-m3next.stage1` handoff may use
`--m3-profile=j613-25g83`. This selects `/opt/mesa-m3/25g83` and
`j613-25g83-hal200`. Its checker reports OpenGL readback and **Vulkan
unavailable**. J615 native25 is experimental (below). Native scratch shaders are
refused; power calibration remains experimental. Legacy 14/Pro Vulkan
stays separate. Firmware and calibration remain specific to each Mac;
the installer migrates neither firmware nor stage 1.

## J615 on 26.6.2 (experimental)

Not yet booted on a J615. The J615 runs the same 26.6.2 DCP, GPU and PMP
firmware as the J613; its handoff is derived from the J615 device tree in
Apple's 26.6.2 restore image. The profile keeps its 25G83 ABI name
(`j613-25g83`, `j613-25g83-hal200`).

Prerequisites:

- J615 booted from its own 26.6.2/25G83 volume group;
- the J615-capable stage 1 `v1.6.1-m3air25.stage1` (the J613-only
  `v1.6.1-m3next.stage1` refuses a J615);
- a matched installer whose release manifest lists `j615` in
  `native25_boards`. Other releases refuse.

```sh
bash install-aurora-sep.sh --m3-profile=j615-25g83
```

This is the J613 25G83 install plus m1n1's
`chosen.asahi,j615-25g83-experimental=1`, which m1n1 and the kernel both
require before the J615 display or GPU is handed over. `--m3-gpu` and
`--m3-profile=j613-25g83` never select it on a J615.

Expect the same result as the J613: OpenGL readback, Vulkan unavailable.
Any identity mismatch leaves the GPU off and the desktop on the boot
framebuffer. **Aurora previous (GPU off)** stays in the boot menu.
Speakers are unchanged by this profile.

Report either way with `aurora-m3-gpu-check --details` and the same
installer's `--m3-report` output.
