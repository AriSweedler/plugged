import Foundation
import IOKit

typealias IOEntry = io_registry_entry_t

func ioProperties(_ entry: IOEntry) -> [String: Any] {
    var dict: Unmanaged<CFMutableDictionary>?
    guard IORegistryEntryCreateCFProperties(entry, &dict, kCFAllocatorDefault, 0) == KERN_SUCCESS,
          let raw = dict?.takeRetainedValue() else { return [:] }
    return (raw as NSDictionary) as? [String: Any] ?? [:]
}

func ioEntryName(_ entry: IOEntry) -> String {
    var buffer = [CChar](repeating: 0, count: 128)
    guard IORegistryEntryGetName(entry, &buffer) == KERN_SUCCESS else { return "" }
    return cString(buffer)
}

func ioEntryID(_ entry: IOEntry) -> UInt64 {
    var id: UInt64 = 0
    IORegistryEntryGetRegistryEntryID(entry, &id)
    return id
}

// The caller releases the returned entry.
func ioParent(_ entry: IOEntry, plane: String) -> IOEntry? {
    var parent: IOEntry = 0
    guard IORegistryEntryGetParentEntry(entry, plane, &parent) == KERN_SUCCESS, parent != 0 else { return nil }
    return parent
}

func ioForEachService(matching className: String, _ body: (IOEntry) -> Void) {
    var iterator: io_iterator_t = 0
    guard IOServiceGetMatchingServices(kIOMainPortDefault, IOServiceMatching(className), &iterator) == KERN_SUCCESS
    else { return }
    ioDrain(iterator, body)
}

func ioForEachChild(of entry: IOEntry, plane: String, _ body: (IOEntry) -> Void) {
    var iterator: io_iterator_t = 0
    guard IORegistryEntryGetChildIterator(entry, plane, &iterator) == KERN_SUCCESS else { return }
    ioDrain(iterator, body)
}

private func ioDrain(_ iterator: io_iterator_t, _ body: (IOEntry) -> Void) {
    defer { IOObjectRelease(iterator) }
    while true {
        let entry = IOIteratorNext(iterator)
        if entry == 0 { return }
        body(entry)
        IOObjectRelease(entry)
    }
}

func cString(_ buffer: [CChar]) -> String {
    String(decoding: buffer.prefix { $0 != 0 }.map { UInt8(bitPattern: $0) }, as: UTF8.self)
}

extension Dictionary where Key == String, Value == Any {
    func int(_ key: String) -> Int? { (self[key] as? NSNumber)?.intValue }
    func bool(_ key: String) -> Bool? { (self[key] as? NSNumber)?.boolValue }
    func string(_ key: String) -> String? { self[key] as? String }
    func strings(_ key: String) -> [String] { self[key] as? [String] ?? [] }
    func dict(_ key: String) -> [String: Any]? { self[key] as? [String: Any] }
}
