# plugged

A dashboard of what is plugged into this machine: the USB-C and HDMI ports, the
USB device tree, the displays, the network interfaces, and power. On a Mac it reads the IORegistry
through IOKit and the display list through AppKit; on Linux it reads sysfs. No
subprocesses, no `system_profiler`, no parsing of `ioreg` output. `plugged --json`
prints the same data once and exits. What every platform must report, and how
that is tested: [docs/SPEC.md](docs/SPEC.md).

```
$ plugged
 plugged  mac                                                    14:20:00  ·  stdin

╭─ Ports ──────────────────────────────────────────────────────────────────────────╮
│ USB-C 1  ●  USB2 USB3  USB3.1 Hub                                                │
│ USB-C 2  ○                                                                       │
│ USB-C 4  ●                         power-in 94 W ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓ │
│ HDMI 3   ○                                                                     ┃ │
├─ Power ────────────────────────────────────────────────────────────────────────╂─┤
│ ██████████████░░░░░░░░░░  57%  charging, 330 min to full                       ┃ │
│ battery  61.2 W  ·  12.24 V × 5 A  ·  in                                       ┃ │
│ adapter  94 W  ·  20 V × 4.7 A  ·  96W USB-C Power Adapter  ·  on USB-C 4 ◀━━━━┛ │
├─ Devices ────────────────────────────────────────────────────────────────────────┤
│ USB3.1 Hub       5 Gb/s    USB-C 1                                               │
│ ├─ Portable SSD  10 Gb/s                                                         │
│ └─ Keyboard      1.5 Mb/s                                                        │
├─ Displays ───────────────────────────────────────────────────────────────────────┤
│ Built-in Retina Display  3024x1964  120 Hz  builtin main                         │
╰──────────────────────────────────────────────────────────────────────────────────╯
```

The port that feeds the machine is joined to the Power panel, which names the adapter's
watts, volts and amps. A second line, blue, joins the device the network comes in on to
its interface in the Network panel (`tests/fixtures/macbook-dock`, a machine the tests
render and check):

```
├─ Devices ────────────────────────────────────────────────────────────────────────┤
│ Portable SSD                   Example · 1 TB           5 Gb/s    bus 2          │
│ USB3.1 Hub                     GenesysLogic             5 Gb/s    bus 6          │
│ └─ USB 10/100/1000 LAN         Realtek                  5 Gb/s           enp6s ┓ │
├─ Network ──────────────────────────────────────────────────────────────────────╂─┤
│ Ethernet  enp0s31f6   ○                                                        ┃ │
│ Ethernet  enp6s0u2u1  ●  1 Gb/s  on USB 10/100/1000 LAN ◀━━━━━━━━━━━━━━━━━━━━━━┛ │
│ Wi-Fi     wlp3s0      ●                                                          │
```

Off a terminal, or with `--text`, the same data is plain text:

```
$ plugged --text
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
| `plugged [--interval SECONDS]` | live dashboard, re-collected every 2 s by default; `q` or Ctrl-C quits; off a terminal, the text view once |
| `plugged --json` | one JSON object with everything, then exit; for scripts |
| `plugged --text` | the same data as the aligned text view above, then exit |
| `plugged tui -` | render one JSON document from stdin once and exit; a piped stdin implies `-` |

`plugged --json | plugged tui -` draws one colored frame when stdout is a
terminal and prints the text view when it is not, so the dashboard can be fed
from a file or another machine. A collection takes well under 100 ms.

## Build

Rust, with three crates: `libc`, `serde`, `serde_json`. Needs `cargo` on `PATH`, or Nix,
from which `plugged-dev` borrows one for the build; and, for `plugged-dev check`, `jq`.

```
bin/plugged --text      # builds on first run, and whenever a source is newer than the binary
bin/plugged-dev check   # build, jq-validate the JSON, round-trip it through `tui -`, render text, run the spec's tests, end with [OK]
```

Put `bin/plugged` on your `PATH` or symlink it there. Nothing is installed
anywhere else; the binary lives in `$XDG_CACHE_HOME/plugged/target/release/`.

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
| `network[]` | `name` (`enp125s0u1`), `kind` (`ethernet`, `wi-fi`, `other`), `connected` (a live link), `speedMbps`, `usbLocationId` when the interface is on a USB device: the `usb[].locationId` it rides |
| `power` | `externalConnected`, `charging`, `fullyCharged`, `percent`, `timeRemainingMin`, `adapter` (`watts`, `volts`, `amps`, `description`) |

Optional keys are omitted, not null.

A USB 3 hub enumerates as two devices, its USB 2 half and its USB 3 half, that
share one `containerId`. The JSON keeps both, as the bus sees them; the text
view and the dashboard fold siblings with the same `containerId` into the
faster half and pool their children.

## Where the data comes from

On a Mac:

| subsystem | source |
|---|---|
| ports | `IOPort` entries, their `IOPortTransportState*` children for the active DisplayPort link rate, and the `Power In` feature's power-source children for the live contract |
| usb | `IOUSBHostDevice` entries; parent links in the `IOUSB` plane form the tree; `kUSBContainerID` is the container id; the top byte of `locationID` is the host-controller bus, and bus N sits behind built-in port N+1 |
| displays | `NSScreen.screens`, `CGDisplayCopyDisplayMode`, `CGDisplayIsBuiltin`, `CGDisplayIsMain` |
| power | `AppleSmartBattery`; adapter watts are volts x amps when the controller reports no `Watts` |
| network | `IONetworkInterface` entries, so the loopback, bridges and tunnels are left out, as are the ones marked `HiddenInterface` or `HiddenConfiguration`; the controller above each has `IOLinkStatus`, the live link, and `IOLinkSpeed`, its rate; an `IO80211` or `IOUserNetworkWLAN` ancestor marks Wi-Fi, and an `IOUSBHostDevice` ancestor is the adapter the interface rides |

On Linux, under `/sys`:

| subsystem | source |
|---|---|
| ports | none: `ports` is empty |
| usb | `bus/usb/devices/<bus>-<port>.<port>…`; the name is the tree; root hubs and interfaces are left out |
| displays | `class/drm/card*-*` that are connected and enabled; the name, size and refresh rate come from the EDID, the size from `modes` without one |
| power | `class/power_supply`: the first `Battery` that is not a peripheral's, and any other supply that is `online` |
| network | `class/net/*` that have a `device` link, so the loopback, bridges and veths are left out; `carrier` is the live link, `speed` its rate, `phy80211` marks Wi-Fi; an adapter's interface shows up under its USB device at `<bus>-<port>:<config>.<interface>/net/<name>` |

## Layout

```
Cargo.toml, Cargo.lock
src/
  main.rs               argument handling and output
  model.rs              the snapshot's structs, which are the JSON
  collect/
    mod.rs              collect_snapshot(): picks the platform's collectors
    mac/                macOS: ioreg.rs (the IOKit and CoreFoundation calls), ports.rs, usb.rs, power.rs, displays.rs
    sysfs*.rs           Linux: sysfs.rs (file readers), sysfs_usb.rs, sysfs_power.rs, sysfs_displays.rs, sysfs_net.rs
  render/
    merge.rs            merge_containers(), folds a hub's two halves
    text.rs             render_text(), the plain view
    frame.rs            build_frame(), the styled view as spans, plus truncate and ANSI output
  tui/
    terminal.rs         window size, raw mode, alternate screen, the signal pipe
    dashboard.rs        argument parsing, the stdin one-shot, the live loop
docs/SPEC.md            what every platform must report
tests/                  conform.jq (the spec's rules), fixtures/ (recorded machines and their expected output)
bin/plugged             build if stale, then run
bin/plugged-dev         build | test | check
```

## License

MIT, see `LICENSE`.
