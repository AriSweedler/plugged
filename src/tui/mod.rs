//! The dashboard: raw ANSI on the alternate screen, no curses and no framework.

mod dashboard;
mod terminal;

pub use dashboard::{parse_options, run_dashboard, run_tui, Options};
