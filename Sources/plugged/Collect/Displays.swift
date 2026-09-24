import AppKit
import CoreGraphics

@MainActor
func collectDisplays() -> [Display] {
    NSScreen.screens.compactMap { screen in
        guard let id = (screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.uint32Value
        else { return nil }
        let mode = CGDisplayCopyDisplayMode(id)
        return Display(
            name: screen.localizedName,
            width: mode?.pixelWidth ?? Int(screen.frame.width),
            height: mode?.pixelHeight ?? Int(screen.frame.height),
            hz: mode?.refreshRate ?? 0,
            builtin: CGDisplayIsBuiltin(id) != 0,
            main: CGDisplayIsMain(id) != 0)
    }
}
