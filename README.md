# plugged

What is plugged into this Mac, in one call: the USB-C and HDMI ports, the USB
device tree, the displays, and power. It reads the IORegistry directly through
IOKit and the display list through AppKit. No subprocesses, no `system_profiler`,
no parsing of `ioreg` output. The same binary renders a live dashboard.

```
$ plugged text
Ports
  USB-C 1  ○  -
  USB-C 2  ●  USB2                   YubiKey FIDO+CCID
  USB-C 3  ●  USB2 USB3 DisplayPort  Anker USB-C Hub Device  power-in
  HDMI 1   ○  -

USB
  YubiKey FIDO+CCID              12 Mb/s   USB-C 2
  Anker USB-C Hub Device         5 Gb/s    USB-C 3
    Freestyle Edge Keyboard      12 Mb/s
    Advanced Corded Mouse M500s  12 Mb/s
    Anker                        480 Mb/s
    USB 10_100_1000 LAN          5 Gb/s
    USB3.0 Card Reader           5 Gb/s

Displays
  LG ULTRAWIDE  3440x1440 @ 50 Hz  main

Power
  battery 100%  charging
  adapter 48 W  20 V x 2.4 A  pd charger
```

## Commands

| command | does |
|---|---|
| `plugged` or `plugged json` | one JSON object with everything, for dashboards and scripts |
| `plugged text` | the same data as the aligned text view above |
| `plugged tui [--interval SECONDS]` | live dashboard, re-collected every 2 s by default; `q` or Ctrl-C quits |
| `plugged tui -` | render one JSON document from stdin once and exit; a piped stdin implies `-` |

`plugged json | plugged tui -` draws one colored frame when stdout is a
terminal and prints the text view when it is not, so the dashboard can be fed
from a file or another machine. A collection takes well under 100 ms.

## Build

Needs Xcode's Swift toolchain (Swift 6, macOS 14 or later) and, for
`plugged-dev check`, `jq`.

```
bin/plugged text        # builds on first run, and whenever a source is newer than the binary
bin/plugged-dev check   # build, jq-validate the JSON, round-trip it through `tui -`, render text, end with [OK]
```

Put `bin/plugged` on your `PATH` or symlink it there. Nothing is installed
anywhere else; the binary lives in `.build/release/`.

## The dashboard

`plugged tui` is raw ANSI, no curses and no framework, written to be at home
inside tmux:

- enters the alternate screen and hides the cursor, and restores both on `q`,
  Ctrl-C, SIGTERM and SIGHUP;
- redraws without clearing the screen: home, every line followed by
  clear-to-end-of-line, then clear-below, so nothing flickers;
- takes the width and height from `ioctl(TIOCGWINSZ)`, re-reads them on
  SIGWINCH and truncates every line to the width;
- uses only the 16 ANSI colors plus bold and dim, never RGB, and only
  single-cell glyphs;
- puts the terminal in non-canonical, no-echo mode only to read keys, keeping
  ISIG so Ctrl-C still arrives as SIGINT, and restores the original termios on
  exit;
- never enables mouse reporting.

Signals are turned into bytes on a pipe that the main loop `poll()`s together
with stdin, so a keypress, a resize, a signal and the next tick all wake the
same loop.

## JSON

`schemaVersion` is 1. Keys are sorted, so two snapshots diff cleanly.

| key | contents |
|---|---|
| `generatedAt` | ISO 8601 with the local offset |
| `host` | `gethostname()` |
| `ports[]` | `id` (`Port-USB-C@3`), `kind` (`usb-c` or `hdmi`), `label`, `active`, `transports` (`USB2`, `USB3`, `DisplayPort`, `CIO`), `displayPortLinkRate` when a DisplayPort link is up, `powerIn` when the port can take power in (true only while a power contract is live on it) |
| `usb[]` | a tree of `name`, `vendor`, `vendorId`, `productId`, `revision`, `serial`, `speed` (0 low to 5 super+ 20 Gb/s), `speedLabel`, `locationId`, `containerId`, `port` (the `ports[].id` the device hangs off), `children` |
| `displays[]` | `name`, `width`, `height` (pixels), `hz`, `builtin`, `main` |
| `power` | `externalConnected`, `charging`, `fullyCharged`, `percent`, `timeRemainingMin`, `adapter` (`watts`, `volts`, `amps`, `description`) |

Optional keys are omitted, not null.

A USB 3 hub enumerates as two devices, its USB 2 half and its USB 3 half, that
share one `containerId`. The JSON keeps both, as the bus sees them; the text
view and the dashboard fold siblings with the same `containerId` into the
faster half and pool their children.

## Where the data comes from

| subsystem | IOKit source |
|---|---|
| ports | `IOPort` entries, their `IOPortTransportState*` children for the active DisplayPort link rate, and the `Power In` feature's power-source children for the live contract |
| usb | `IOUSBHostDevice` entries; parent links in the `IOUSB` plane form the tree; `kUSBContainerID` is the container id; the top byte of `locationID` is the host-controller bus, and bus N sits behind built-in port N+1 |
| displays | `NSScreen.screens`, `CGDisplayCopyDisplayMode`, `CGDisplayIsBuiltin`, `CGDisplayIsMain` |
| power | `AppleSmartBattery`; adapter watts are volts x amps when the controller reports no `Watts` |

## Layout

```
Package.swift
Sources/plugged/
  main.swift            argument handling and output
  IOKitHelpers.swift    thin wrappers over the IOKit C calls
  Model.swift           the Codable structs behind the JSON
  Collect/              one file per subsystem, one entry function each
    Snapshot.swift      collectSnapshot(), which calls the four below
    USB.swift           collectUSB()
    TypeC.swift         collectPorts()
    Power.swift         collectPower()
    Displays.swift      collectDisplays()
  Render/
    Merge.swift         mergeContainers(_:), folds a hub's two halves
    Text.swift          renderText(_:), the plain view
    Frame.swift         buildFrame(_:status:), the styled view as spans, plus truncate and ANSI output
  TUI/
    Terminal.swift      window size, raw mode, alternate screen, the signal pipe
    Dashboard.swift     `tui` argument parsing, the stdin one-shot, the live loop
bin/plugged             build if stale, then run
bin/plugged-dev         build | check
```

## License

MIT, see `LICENSE`.
