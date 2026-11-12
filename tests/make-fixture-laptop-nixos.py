#!/usr/bin/env python3
"""Writes tests/fixtures/laptop-nixos/sys: a sysfs tree shaped like a 2017 MacBook Pro under
NixOS on mains, with a dock, a keyboard and a 4K monitor. Serials and EDIDs are made up.
Run it again after editing; then `bin/plugged-dev test --update` rewrites the expected files."""
import pathlib, shutil, struct

ROOT = pathlib.Path(__file__).parent / "fixtures" / "laptop-nixos" / "sys"


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
