//! plugged: a dashboard of what is plugged into this machine. docs/SPEC.md is the contract.

mod collect;
mod model;
mod render;
mod tui;

use std::process::exit;

const USAGE: &str = "\
usage: plugged [--interval SECONDS] | plugged --json | plugged --text | plugged tui [--interval SECONDS] [-]
  (no flag)  the dashboard of what is plugged in, re-collected every SECONDS (default 2);
             q or Ctrl-C quits. Not on a terminal: the text view, once
  --json     one JSON object with everything plugged in, then exit (also: json)
  --text     the same data as an aligned text view, then exit (also: text)
  tui        the dashboard; with - (or a piped stdin) it renders one JSON document from
             stdin and exits; --width COLUMNS draws that frame COLUMNS wide, uncolored off a terminal";

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let first = arguments.first().map(String::as_str);
    let code = match first {
        None => tui::run_dashboard(&options(&arguments, "")),
        Some(flag) if flag == "--interval" || flag.starts_with("--interval=") => tui::run_dashboard(&options(&arguments, "")),
        Some("--json" | "json") => {
            // Through a Value, whose maps are sorted: the keys come out in one order everywhere.
            let document = serde_json::to_value(collect::collect_snapshot()).expect("a snapshot is plain data");
            println!("{}", serde_json::to_string_pretty(&document).expect("a snapshot is plain data"));
            0
        }
        Some("--text" | "text") => {
            println!("{}", render::render_text(&collect::collect_snapshot()));
            0
        }
        Some("tui") => tui::run_tui(&options(&arguments[1..], "tui ")),
        Some("-h" | "--help" | "help") => {
            println!("{USAGE}");
            0
        }
        Some(command) => {
            eprintln!("plugged: unknown command '{command}'\n{USAGE}");
            2
        }
    };
    exit(code);
}

fn options(arguments: &[String], what: &str) -> tui::Options {
    tui::parse_options(arguments).unwrap_or_else(|| {
        eprintln!("plugged: bad {what}arguments\n{USAGE}");
        exit(2)
    })
}
