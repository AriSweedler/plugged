# The checks of docs/SPEC.md that hold for every snapshot, whatever platform wrote it:
#   plugged json | jq -e -f tests/conform.jq
# Prints true and exits 0, or stops at the first broken rule with its name.
def must(rule; name): if rule then . else error("spec: " + name) end;
def is_bool: type == "boolean";
def is_int: type == "number" and . == floor;
def devices: .[] | ., (.children | devices);

must(.schemaVersion == 1; "schemaVersion is 1")
| must(.generatedAt | test("^\\d{4}-\\d\\d-\\d\\dT\\d\\d:\\d\\d:\\d\\d(Z|[+-]\\d\\d:\\d\\d)$"); "generatedAt is ISO 8601 with a zone")
| must(.host | type == "string"; "host is a string")
| must([.ports, .usb, .displays] | all(type == "array"); "ports, usb and displays are arrays")
| must([.ports[].id] | length == (unique | length); "port ids are unique")
| must(.ports | all(
    (.kind | IN("usb-c", "hdmi")) and (.label | type == "string") and (.active | is_bool)
    and (.transports | type == "array") and (.powerIn | is_bool or . == null)); "every port has kind, label, active, transports, powerIn")
| must([.usb | devices] | all(
    (.name | type == "string" and length > 0) and (.vendor | type == "string")
    and (.vendorId | is_int) and (.productId | is_int)
    and (.speed | is_int and . >= -1 and . <= 5) and (.speedLabel | type == "string")
    and (.locationId | test("^0x[0-9a-f]{8}$")) and (.port | type == "string")
    and (.children | type == "array")); "every usb device, at any depth, has its fields")
| must([.usb | devices | .children | [.[].locationId] | . == sort] | all; "children are sorted by locationId")
| must((.ports | length) == 0 or ([.ports[].id] as $ids | [.usb[].port] | all(IN($ids[]))); "when ports are reported, a root device's port is a port id")
| must(.displays | all(
    (.name | type == "string" and length > 0) and (.width | is_int and . > 0) and (.height | is_int and . > 0)
    and (.hz | type == "number" and . >= 0) and (.builtin | is_bool) and (.main | is_bool)); "every display has a name, a size, hz, builtin, main")
| must([.displays[] | select(.main)] | length <= 1; "at most one main display")
| must(.power | (.externalConnected | is_bool) and (.charging | is_bool) and (.fullyCharged | is_bool)
    and (.percent | is_int and . >= 0 and . <= 100); "power has its three flags and a 0-100 percent")
| must(.power | (.charging and .fullyCharged) | not; "a battery is not charging and full at once")
| must(.power | (.charging | not) or .externalConnected; "a charging battery has external power")
| must(.power.timeRemainingMin | . == null or (is_int and . > 0); "timeRemainingMin is absent or positive")
| must(.power.adapter | . == null or ((.watts | is_int and . > 0) and (.volts | type == "number") and (.amps | type == "number") and (.description | type == "string")); "adapter is absent or has watts, volts, amps, description")
| must(.power.battery | . == null or ((.watts | type == "number" and . > 0) and (.volts | type == "number" and . > 0) and (.amps | type == "number" and . > 0)); "battery flow is absent or has positive watts, volts, amps")
| must(([.ports[] | select(.powerIn == true)] | length) == 0 or .power.externalConnected; "a port that feeds the machine means external power")
| true
