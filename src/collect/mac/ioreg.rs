//! The few calls of IOKit and CoreFoundation the collectors need, declared by hand, and a
//! registry entry's property table as plain Rust values.

use std::collections::HashMap;
use std::ffi::{c_char, c_void, CStr, CString};

pub type CFTypeRef = *const c_void;
pub type Entry = u32;

const UTF8: u32 = 0x0800_0100; // kCFStringEncodingUTF8
const SINT64: isize = 4; // kCFNumberSInt64Type
const FLOAT64: isize = 6; // kCFNumberFloat64Type

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFGetTypeID(cf: CFTypeRef) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFNumberGetTypeID() -> usize;
    fn CFBooleanGetTypeID() -> usize;
    fn CFArrayGetTypeID() -> usize;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFRelease(cf: CFTypeRef);
    fn CFStringGetLength(string: CFTypeRef) -> isize;
    fn CFStringGetMaximumSizeForEncoding(length: isize, encoding: u32) -> isize;
    fn CFStringGetCString(string: CFTypeRef, buffer: *mut c_char, size: isize, encoding: u32) -> u8;
    fn CFNumberIsFloatType(number: CFTypeRef) -> u8;
    fn CFNumberGetValue(number: CFTypeRef, kind: isize, value: *mut c_void) -> u8;
    fn CFBooleanGetValue(boolean: CFTypeRef) -> u8;
    fn CFArrayGetCount(array: CFTypeRef) -> isize;
    fn CFArrayGetValueAtIndex(array: CFTypeRef, index: isize) -> CFTypeRef;
    fn CFDictionaryGetCount(dictionary: CFTypeRef) -> isize;
    fn CFDictionaryGetKeysAndValues(dictionary: CFTypeRef, keys: *mut CFTypeRef, values: *mut CFTypeRef);
}

#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOServiceMatching(name: *const c_char) -> CFTypeRef;
    fn IOServiceGetMatchingServices(main_port: u32, matching: CFTypeRef, existing: *mut u32) -> i32;
    fn IOIteratorNext(iterator: u32) -> u32;
    fn IOObjectRelease(object: u32) -> i32;
    fn IORegistryEntryCreateCFProperties(entry: u32, properties: *mut CFTypeRef, allocator: CFTypeRef, options: u32) -> i32;
    fn IORegistryEntryGetName(entry: u32, name: *mut c_char) -> i32;
    fn IORegistryEntryGetRegistryEntryID(entry: u32, id: *mut u64) -> i32;
    fn IORegistryEntryGetParentEntry(entry: u32, plane: *const c_char, parent: *mut u32) -> i32;
    fn IORegistryEntryGetChildIterator(entry: u32, plane: *const c_char, iterator: *mut u32) -> i32;
}

/// A property's value. Data, dates and anything else the collectors never read are Other:
/// present, with nothing to say.
pub enum Value {
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Array(Vec<Value>),
    Dict(Props),
    Other,
}

#[derive(Default)]
pub struct Props(HashMap<String, Value>);

impl Props {
    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    // A boolean is a number to the registry and a number is a boolean, as NSNumber has it.
    pub fn int(&self, key: &str) -> Option<i64> {
        match self.0.get(key)? {
            Value::Int(value) => Some(*value),
            Value::Float(value) => Some(*value as i64),
            Value::Bool(value) => Some(*value as i64),
            _ => None,
        }
    }

    pub fn bool(&self, key: &str) -> Option<bool> {
        match self.0.get(key)? {
            Value::Bool(value) => Some(*value),
            Value::Int(value) => Some(*value != 0),
            Value::Float(value) => Some(*value != 0.0),
            _ => None,
        }
    }

    pub fn string(&self, key: &str) -> Option<String> {
        match self.0.get(key)? {
            Value::Str(value) => Some(value.clone()),
            _ => None,
        }
    }

    pub fn strings(&self, key: &str) -> Vec<String> {
        match self.0.get(key) {
            Some(Value::Array(values)) => values.iter().filter_map(|value| if let Value::Str(text) = value { Some(text.clone()) } else { None }).collect(),
            _ => Vec::new(),
        }
    }

    pub fn dict(&self, key: &str) -> Option<&Props> {
        match self.0.get(key)? {
            Value::Dict(value) => Some(value),
            _ => None,
        }
    }
}

/// Borrows `cf`; the caller still owns it.
pub fn value(cf: CFTypeRef) -> Value {
    if cf.is_null() {
        return Value::Other;
    }
    unsafe {
        let kind = CFGetTypeID(cf);
        if kind == CFStringGetTypeID() {
            Value::Str(string(cf))
        } else if kind == CFBooleanGetTypeID() {
            Value::Bool(CFBooleanGetValue(cf) != 0)
        } else if kind == CFNumberGetTypeID() {
            if CFNumberIsFloatType(cf) != 0 {
                let mut number = 0f64;
                CFNumberGetValue(cf, FLOAT64, (&mut number as *mut f64).cast());
                Value::Float(number)
            } else {
                let mut number = 0i64;
                CFNumberGetValue(cf, SINT64, (&mut number as *mut i64).cast());
                Value::Int(number)
            }
        } else if kind == CFArrayGetTypeID() {
            Value::Array((0..CFArrayGetCount(cf)).map(|index| value(CFArrayGetValueAtIndex(cf, index))).collect())
        } else if kind == CFDictionaryGetTypeID() {
            Value::Dict(dictionary(cf))
        } else {
            Value::Other
        }
    }
}

/// An array's elements, borrowed, for elements that are not property values.
pub fn items(array: CFTypeRef) -> Vec<CFTypeRef> {
    if array.is_null() {
        return Vec::new();
    }
    unsafe { (0..CFArrayGetCount(array)).map(|index| CFArrayGetValueAtIndex(array, index)).collect() }
}

pub fn dictionary(cf: CFTypeRef) -> Props {
    unsafe {
        let count = CFDictionaryGetCount(cf) as usize;
        let mut keys: Vec<CFTypeRef> = vec![std::ptr::null(); count];
        let mut values: Vec<CFTypeRef> = vec![std::ptr::null(); count];
        CFDictionaryGetKeysAndValues(cf, keys.as_mut_ptr(), values.as_mut_ptr());
        let mut props = HashMap::with_capacity(count);
        for (key, item) in keys.into_iter().zip(values) {
            if CFGetTypeID(key) == CFStringGetTypeID() {
                props.insert(string(key), value(item));
            }
        }
        Props(props)
    }
}

unsafe fn string(cf: CFTypeRef) -> String {
    let size = CFStringGetMaximumSizeForEncoding(CFStringGetLength(cf), UTF8) + 1;
    let mut buffer = vec![0u8; size as usize];
    if CFStringGetCString(cf, buffer.as_mut_ptr().cast(), size, UTF8) == 0 {
        return String::new();
    }
    CStr::from_bytes_until_nul(&buffer).map(|text| text.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn properties(entry: Entry) -> Props {
    let mut table: CFTypeRef = std::ptr::null();
    unsafe {
        if IORegistryEntryCreateCFProperties(entry, &mut table, std::ptr::null(), 0) != 0 || table.is_null() {
            return Props::default();
        }
        let props = dictionary(table);
        CFRelease(table);
        props
    }
}

pub fn entry_name(entry: Entry) -> String {
    let mut buffer = [0u8; 128];
    if unsafe { IORegistryEntryGetName(entry, buffer.as_mut_ptr().cast()) } != 0 {
        return String::new();
    }
    CStr::from_bytes_until_nul(&buffer).map(|text| text.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn entry_id(entry: Entry) -> u64 {
    let mut id = 0u64;
    unsafe { IORegistryEntryGetRegistryEntryID(entry, &mut id) };
    id
}

pub fn parent_id(entry: Entry, plane: &str) -> Option<u64> {
    let plane = CString::new(plane).ok()?;
    let mut parent: Entry = 0;
    unsafe {
        if IORegistryEntryGetParentEntry(entry, plane.as_ptr(), &mut parent) != 0 || parent == 0 {
            return None;
        }
        let id = entry_id(parent);
        IOObjectRelease(parent);
        Some(id)
    }
}

pub fn for_each_service(class_name: &str, body: impl FnMut(Entry)) {
    let Ok(class_name) = CString::new(class_name) else { return };
    let mut iterator = 0u32;
    // IOServiceGetMatchingServices consumes the matching dictionary.
    if unsafe { IOServiceGetMatchingServices(0, IOServiceMatching(class_name.as_ptr()), &mut iterator) } != 0 {
        return;
    }
    drain(iterator, body);
}

pub fn for_each_child(entry: Entry, plane: &str, body: impl FnMut(Entry)) {
    let Ok(plane) = CString::new(plane) else { return };
    let mut iterator = 0u32;
    if unsafe { IORegistryEntryGetChildIterator(entry, plane.as_ptr(), &mut iterator) } != 0 {
        return;
    }
    drain(iterator, body);
}

fn drain(iterator: u32, mut body: impl FnMut(Entry)) {
    loop {
        let entry = unsafe { IOIteratorNext(iterator) };
        if entry == 0 {
            break;
        }
        body(entry);
        unsafe { IOObjectRelease(entry) };
    }
    unsafe { IOObjectRelease(iterator) };
}
