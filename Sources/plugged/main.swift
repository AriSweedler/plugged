import Foundation

let usage = """
    usage: plugged [json|text|tui [--interval SECONDS] [-]]
      json  one JSON object with everything plugged in (default)
      text  the same data as an aligned text view
      tui   live dashboard, re-collected every SECONDS (default 2); q or Ctrl-C quits
            with - (or a piped stdin) it renders one JSON document from stdin and exits
    """

let arguments = CommandLine.arguments.dropFirst()
switch arguments.first ?? "json" {
case "json":
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.prettyPrinted, .sortedKeys, .withoutEscapingSlashes]
    print(String(decoding: try encoder.encode(collectSnapshot()), as: UTF8.self))
case "text":
    print(renderText(collectSnapshot()))
case "tui":
    guard let options = parseTUIOptions(arguments.dropFirst()) else {
        FileHandle.standardError.write(Data("plugged: bad tui arguments\n\(usage)\n".utf8))
        exit(2)
    }
    exit(runTUI(options))
case "-h", "--help", "help":
    print(usage)
case let command:
    FileHandle.standardError.write(Data("plugged: unknown command '\(command)'\n\(usage)\n".utf8))
    exit(2)
}
