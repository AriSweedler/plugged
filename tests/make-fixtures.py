#!/usr/bin/env python3
"""Writes the recorded machines under tests/fixtures/<name>/sys: sysfs trees shaped like a 2017
MacBook Pro under NixOS. Serials and EDIDs are made up.

  laptop-nixos   on mains, a dock's hub, a keyboard, a 4K monitor, Wi-Fi up and a built-in
                 Ethernet port with no cable in it; its SMC cannot be read
  macbook-dock   the same, fed through a dock on two ports, with a USB disk and the dock's
                 Ethernet adapter carrying the network; its SMC can be read, so the ports, the
                 port in use and the adapter are known

Run it again after editing; then `bin/plugged-dev test --update` rewrites the expected files."""
import pathlib, shutil, struct

FIXTURES = pathlib.Path(__file__).parent / "fixtures"
ROOT = FIXTURES / "laptop-nixos" / "sys"


def put(directory: str, **attributes: str) -> None:
    for name, value in attributes.items():
        path = ROOT / directory / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(value + "\n")


def edid(name: str, width: int, height: int, hblank: int, vblank: int, hz: float) -> bytes:
    clock = round((width + hblank) * (height + vblank) * hz / 10_000)
    timing = struct.pack("<H", clock) + bytes([
        width & 0xFF, hblank & 0xFF, (width >> 8) << 4 | hblank >> 8,
        height & 0xFF, vblank & 0xFF, (height >> 8) << 4 | vblank >> 8,
    ]) + bytes(10)
    label = bytes([0, 0, 0, 0xFC, 0]) + (name.encode() + b"\n").ljust(13, b" ")
    return bytes(54) + timing + label + bytes(36) + bytes(2)


shutil.rmtree(ROOT, ignore_errors=True)

usb = "bus/usb/devices"
for bus, speed in [(1, "480"), (2, "5000"), (6, "10000")]:  # root hubs: never reported
    put(f"{usb}/usb{bus}", idVendor="1d6b", idProduct="0002", bcdDevice="0618", speed=speed, busnum=str(bus), devpath="0", product="xHCI Host Controller")
put(f"{usb}/1-0:1.0", bInterfaceClass="09")  # an interface: never reported
put(f"{usb}/1-7", idVendor="05e3", idProduct="0610", bcdDevice="0663", speed="480", busnum="1", devpath="7", manufacturer="GenesysLogic", product="USB2.1 Hub")
put(f"{usb}/1-7.4", idVendor="0424", idProduct="2734", bcdDevice="0127", speed="480", busnum="1", devpath="7.4", manufacturer="Microchip Tech", product="USB2734")
put(f"{usb}/1-7.4.2", idVendor="058f", idProduct="9410", bcdDevice="0122", speed="1.5", busnum="1", devpath="7.4.2", manufacturer="KINESIS FREESTYLE KB800", product="KB800 Kinesis Freestyle")
put(f"{usb}/1-7.4.3", idVendor="0bda", idProduct="3035", bcdDevice="0001", speed="480", busnum="1", devpath="7.4.3", manufacturer="Generic", product="4k Webcam", serial="200901010001")
put(f"{usb}/1-7.1", idVendor="291a", idProduct="8355", bcdDevice="0112", speed="12", busnum="1", devpath="7.1", manufacturer="BillBoard", product="USB BillBoard", serial="SN00000000")
put(f"{usb}/1-3", idVendor="05ac", idProduct="1281", bcdDevice="0000", speed="480", busnum="1", devpath="3", manufacturer="Apple Inc.")  # no product string
put(f"{usb}/6-2", idVendor="05e3", idProduct="0626", bcdDevice="0663", speed="5000", busnum="6", devpath="2", manufacturer="GenesysLogic", product="USB3.1 Hub")

# class/net, where a real tree links every interface to the device behind it. Only an interface
# with hardware under it has that link, and the fixture stands it up as a file: the collector
# asks whether it is there, nothing more. The virtual ones below have none, and are skipped.
net = "class/net"
put(f"{net}/enp0s31f6", device="", type="1", carrier="0", speed="-1", operstate="down", address="00:11:22:33:44:55")
put(f"{net}/wlp2s0", device="", phy80211="", type="1", carrier="1", speed="", operstate="up", address="00:11:22:33:44:56")
put(f"{net}/lo", type="772", carrier="1", operstate="unknown", address="00:00:00:00:00:00")
put(f"{net}/docker0", type="1", carrier="0", speed="-1", operstate="down", address="02:42:1a:2b:3c:4d")
put(f"{net}/veth1a2b3c4", type="1", carrier="1", speed="10000", operstate="up", address="02:42:1a:2b:3c:4e")

supply = "class/power_supply"
put(f"{supply}/ADP1", type="Mains", online="1")
put(f"{supply}/BAT0", type="Battery", status="Full", capacity="79", voltage_now="12780000", current_now="0", charge_now="5244000", charge_full="5377000")
put(f"{supply}/apple_mfi_fastcharge_1-2", type="Battery", scope="Device")  # a phone's battery: skipped

drm = "class/drm"
put(f"{drm}/card2-DP-3", status="connected", enabled="enabled", modes="3840x2160\n1920x1080")
(ROOT / drm / "card2-DP-3" / "edid").write_bytes(edid("TEST 4K", 3840, 2160, 160, 62, 60))
put(f"{drm}/card2-eDP-2", status="connected", enabled="enabled", modes="2880x1800")  # no EDID: the modes file answers
put(f"{drm}/card2-DP-1", status="disconnected", enabled="disabled")
put(f"{drm}/card2", dev="226:2")  # the card itself: not a connector

# The second machine: the first, then what differs.
DOCK = FIXTURES / "macbook-dock" / "sys"
shutil.rmtree(DOCK, ignore_errors=True)
shutil.copytree(ROOT, DOCK)
ROOT = DOCK

put(f"{supply}/BAT0", status="Charging", capacity="62", current_now="1500000")
# A disk: a block device under the USB device's interface, sized in 512-byte sectors (1 TB).
put(f"{usb}/2-1", idVendor="1234", idProduct="5678", bcdDevice="0100", speed="5000", busnum="2", devpath="1", manufacturer="Example", product="Portable SSD", serial="SN00000002")
put(f"{usb}/2-1/2-1:1.0/host2/target2:0:0/2:0:0:0/block/sda", size="1953525168")

# The dock's Ethernet adapter, on the USB 3 hub: its interface publishes a net device, the way
# the disk above publishes a block device, which is how an interface is tied to what it rides on.
put(f"{usb}/6-2.1", idVendor="0bda", idProduct="8153", bcdDevice="3000", speed="5000", busnum="6", devpath="2.1", manufacturer="Realtek", product="USB 10/100/1000 LAN", serial="SN00000003")
put(f"{usb}/6-2.1/6-2.1:1.0/net/enp6s0u2u1", ifindex="5")
put(f"{net}/enp6s0u2u1", device="", type="1", carrier="1", speed="1000", operstate="up", address="00:11:22:33:44:57")

# The SMC's keys as raw bytes, where a real machine has the driver's key_at_index files: four
# ports, a 20 V x 4.3 A contract on ports 3 and 4 (a dock on two ports), port 3 in use.
keys = ROOT / "devices/platform/applesmc.768/keys"
keys.mkdir(parents=True)
smc = {"AC-N": bytes([4]), "AC-W": bytes([3]), "D3in": b"87W USB-C Power Adapter".ljust(32, b"\0")}
for port, (millivolts, milliamps) in {1: (0, 0), 2: (0, 0), 3: (20000, 4300), 4: (20000, 4250)}.items():
    smc[f"D{port}VR"] = struct.pack(">H", millivolts)
    smc[f"D{port}IR"] = struct.pack(">H", milliamps)
for name, data in smc.items():
    (keys / name).write_bytes(data)
