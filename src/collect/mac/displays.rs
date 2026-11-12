use super::ioreg::{self, CFTypeRef, Value};
use crate::model::Display;
use std::ffi::{c_char, c_void};

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGDisplayCopyDisplayMode(display: u32) -> CFTypeRef;
    fn CGDisplayModeGetPixelWidth(mode: CFTypeRef) -> usize;
    fn CGDisplayModeGetPixelHeight(mode: CFTypeRef) -> usize;
    fn CGDisplayModeGetRefreshRate(mode: CFTypeRef) -> f64;
    fn CGDisplayModeRelease(mode: CFTypeRef);
    fn CGDisplayPixelsWide(display: u32) -> usize;
    fn CGDisplayPixelsHigh(display: u32) -> usize;
    fn CGDisplayIsBuiltin(display: u32) -> u32;
    fn CGDisplayIsMain(display: u32) -> u32;
}

// NSScreen is the only place a display's name lives. Its class comes from AppKit, reached
// through the Objective-C runtime; what it returns is toll-free bridged to CoreFoundation.
#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[link(name = "objc")]
extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> *mut c_void;
    fn objc_msgSend();
    fn objc_autoreleasePoolPush() -> *mut c_void;
    fn objc_autoreleasePoolPop(pool: *mut c_void);
}

// `[receiver selector]` for a selector with no arguments that returns an object.
unsafe fn send(receiver: *mut c_void, selector: &std::ffi::CStr) -> *mut c_void {
    let call: extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void = std::mem::transmute(objc_msgSend as unsafe extern "C" fn());
    call(receiver, sel_registerName(selector.as_ptr()))
}

pub fn collect() -> Vec<Display> {
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let class = objc_getClass(c"NSScreen".as_ptr());
        let displays = if class.is_null() {
            Vec::new()
        } else {
            ioreg::items(send(class, c"screens").cast_const()).into_iter().filter_map(|screen| display(screen.cast_mut())).collect()
        };
        objc_autoreleasePoolPop(pool);
        displays
    }
}

unsafe fn display(screen: *mut c_void) -> Option<Display> {
    let description = ioreg::dictionary(send(screen, c"deviceDescription").cast_const());
    let id = description.int("NSScreenNumber")? as u32;
    let Value::Str(name) = ioreg::value(send(screen, c"localizedName").cast_const()) else { return None };
    let mode = CGDisplayCopyDisplayMode(id);
    let (width, height, hz) = if mode.is_null() {
        (CGDisplayPixelsWide(id), CGDisplayPixelsHigh(id), 0.0)
    } else {
        let size = (CGDisplayModeGetPixelWidth(mode), CGDisplayModeGetPixelHeight(mode), CGDisplayModeGetRefreshRate(mode));
        CGDisplayModeRelease(mode);
        size
    };
    Some(Display { name, width: width as u32, height: height as u32, hz, builtin: CGDisplayIsBuiltin(id) != 0, main: CGDisplayIsMain(id) != 0 })
}
