# plugged: the spec

`plugged` is one command that shows what is plugged into the machine it runs on. This file is
what every platform's implementation must do; `tests/` checks it.

## Commands

| command | does |
|---|---|
| `plugged [--interval SECONDS]` | the dashboard: collects and redraws every SECONDS (default 2); `q` or Ctrl-C quits. When stdin or stdout is not a terminal it prints the text view once and exits |
| `plugged --json` | collects once, prints one snapshot as JSON, keys sorted, and exits |
| `plugged --text` | collects once, prints the snapshot as text, and exits |
| `plugged tui -`, or `plugged tui` with stdin piped | renders one snapshot read from stdin, from this or any other machine; `--width COLUMNS` draws the dashboard that wide, uncolored when stdout is not a terminal |

`json`, `text` and `tui` without the dashes are the same commands. Bad arguments, an interval
over a day (86400) among them, print the usage and exit 2. A document on stdin that is not a
snapshot, or whose `schemaVersion` this build does not read, is refused in one line on stderr
with exit 1.

A collection reads the kernel directly and starts no other process. Text and the dashboard are
drawn from the snapshot alone, so `plugged json | plugged tui -` shows what `plugged tui` shows.

## The snapshot

One object. A field a platform cannot know takes the value its **Unknown** column names; it is
never guessed and never left out, except the fields marked optional.

| field | meaning | Unknown |
|---|---|---|
| `schemaVersion` | `1`. Raised when a field changes meaning or goes away; a new field does not raise it | |
| `generatedAt` | when it was collected, ISO 8601 with the zone | |
| `host` | the machine's hostname | `""` |
| `ports[]` | the machine's own sockets, in the order they are numbered | `[]` |
| `usb[]` | the devices on the USB buses, as a tree; host controllers and root hubs are not devices | `[]` |
| `displays[]` | the displays that are showing a picture | `[]` |
| `network[]` | the network interfaces the machine has in hardware, sorted by `name` | `[]` |
| `power` | the machine's battery and what feeds it | see below |

**Port**: `id` (unique), `kind` (`usb-c`, `hdmi`), `label` (what a person calls it, `USB-C 1`),
`active` (something is connected), `transports` (what runs over the cable now: `USB2`, `USB3`,
`DisplayPort`, `CIO`), `displayPortLinkRate` (optional), `powerIn` (`true` while the port feeds
the machine, `false` when it could and does not, `null` when it cannot).

**USB device**: `name` (the product string; `vvvv:pppp` in hex when it has none), `vendor`
(`""` when it has none), `vendorId`, `productId`, `revision` (`major.minor`), `serial`
(optional), `speed` (`0` 1.5 Mb/s, `1` 12 Mb/s, `2` 480 Mb/s, `3` 5 Gb/s, `4` 10 Gb/s,
`5` 20 Gb/s, `-1` unknown), `speedLabel` (the same for a person), `locationId` (`0x` and eight
hex digits: the bus in the top byte, then one digit per hub port down to the device), `containerId`
(optional; the two halves of a USB 3 hub share it, and text and the dashboard fold them into
one), `port` (for a device at the root of the tree, the `id` of the port it hangs off when the
platform can tell, else the bus it is on, `bus 1`), `storage` (optional, a disk: `bytes`, and
`volumes`, the names of the volumes on it, `[]` when the platform cannot name them), `children`
(the devices behind this hub, sorted by `locationId`, as the roots are).

The views call a disk by its volumes when it has any, since an enclosure's own product string
is as plain as "External", and put what the device says it is (maker, product, size) beside
that.

**Interface**: `name` (what the kernel calls it, `enp125s0u1`), `kind` (`ethernet`, `wi-fi`,
`other`), `connected` (a cable is in and the link is up, or the radio is associated),
`speedMbps` (the rate the link negotiated, `0` unknown), `usbLocationId` (optional: the
`locationId` of the USB device the interface lives on, so `network[].usbLocationId ==
usb[].locationId` joins them). A virtual interface is not one of these: the loopback, a bridge,
a container's veth and a tunnel are left out, and what is left is what a cable can go into. The
interface a Thunderbolt port keeps for networking with another computer is listed only while
that link is up. A
document written before `network` existed reads as `[]`, so it keeps `schemaVersion` 1.

**Display**: `name`, `width` and `height` of the mode in use or, failing that, the native mode,
`hz` (`0` unknown), `builtin`, `main` (`false` on a platform with no main display; at most one).

**Power**: `externalConnected`, `charging`, `fullyCharged`, `percent` (0 to 100),
`timeRemainingMin` (optional: to empty on battery, to full while charging), `adapter` (optional:
`watts`, `volts`, `amps`, `description`, only when the source says what it negotiated),
`battery` (optional: `watts`, `volts`, `amps` flowing into the battery while `charging`, out of
it otherwise; left out when nothing flows). A machine with no battery reports
`externalConnected: true`, the other flags `false`, `percent: 0`.

Where the power comes in is the port whose `powerIn` is `true`. The views name it beside the
adapter, and the dashboard draws a line from that port's row to the Power panel. A platform
that reports no ports says the port is unknown.

Where the network comes in is the first interface with a live link that rides a USB device. The
views name that device beside the interface and the interface beside the device, and the
dashboard draws a second line, from the device's row to the Network panel. Both lines run down
the same rail in the frame's gutter: power yellow, network blue.

## Platforms

| platform | reads | cannot know |
|---|---|---|
| macOS | the IORegistry (IOKit) and AppKit's screens | a Wi-Fi link's `speedMbps` |
| Linux | sysfs: `bus/usb/devices` (with each device's `port` link and the port's `peer`, which pairs a USB 3 hub's two halves under one `containerId` made of their names, `1-7+6-2`), `class/power_supply`, `class/drm`, `class/net`; udev's database beside it (`/run/udev/data`) for a disk's volume labels | `ports`, `containerId` of any other device, `main`, the name of a volume with no label; `adapter` unless a supply publishes its contract |
| Linux on an Intel Mac | the above, and the SMC through applesmc's `key_at_index` files when the machine lets the user write the index (root only by default) | as Linux, except: `ports` are known for power only (`active` means a source is on the port, `transports` is empty), with the port in use and the `adapter` it negotiated |

A new platform is a set of collectors that fills the snapshot, a branch in
`src/collect/mod.rs`, a row here, and tests.

## Tests

`bin/plugged-dev test`, which `check` and the pre-commit hook run:

- **Conformance.** `tests/conform.jq` holds the rules above that a program can check. The
  snapshot of the machine the tests run on must pass it, so each platform is checked where it
  is built.
- **Recorded machines.** Each `tests/fixtures/<name>/sys` is a sysfs tree, with udev's database
  beside it as `run/udev/data` where the machine has one. `PLUGGED_SYSFS_ROOT`
  points the Linux collectors at it, on any platform, and the result must pass conformance and
  equal `expected.json` (with `generatedAt` and `host` pinned) and, rendered, `expected.txt`
  (the text view) and `expected.frame.txt` (the dashboard, 84 columns, uncolored: what
  `plugged tui - --width 84` prints off a terminal). So a Mac catches a Linux regression, and
  a change to either view shows up as a diff.
- **Recorded snapshots.** Each `tests/snapshots/<name>.json` is a snapshot document, from any
  platform, with `<name>.txt` and `<name>.frame.txt` beside it. `mac-charging` is a Mac fed
  through one of its ports: the case that draws the power link.
- **Unit tests.** `cargo test` runs first, for rendering no snapshot can reach: two links that
  cross, where the one going down stays whole and the one going across breaks.
- `bin/plugged-dev test --update` rewrites the expected files; the diff is the review.

A new fixture is a script like `tests/make-fixtures.py`. Fixtures hold made-up
serials and EDIDs, never a real machine's.
