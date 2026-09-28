---
name: ari-dotfile--submodule-plugged
description: Work on `plugged`, the Swift CLI and TUI that reports what is plugged into this Mac (ports, USB tree, displays, power) from IOKit with no subprocesses. The repo is a dotfiles submodule at ~/.config/plugged; this skill has its commands, layout, hooks and failures. Commits, pushes and pointer bumps follow ari-dotfile--submodule.
---

# plugged in the dotfiles

`plugged` is one Swift package: `plugged json` prints one JSON document,
`plugged text` renders it, `plugged tui` renders it live. It reads the
IORegistry through IOKit and the display list through AppKit; it never spawns a
process. The repo is a dotfiles submodule: `/ari-dotfile--submodule` has the
layout it follows, how to be on `main`, how commits and the user's push flow,
and the generic failures. This skill has what is plugged-specific.

## Layout

| Piece | Path | Tier |
|---|---|---|
| Repo (public) | `~/.config/plugged/`, upstream `github.com/AriSweedler/plugged` | df pointer |
| Driver | `~/.config/bin/plugged -> ../plugged/bin/plugged`; the wrapper builds `release` when a source is newer than the binary, then `exec`s it | df |
| Dev driver | `~/.config/plugged/bin/plugged-dev` (`build`, `check`) | repo |
| Sources | `Sources/plugged/`: `main.swift`, `Model.swift` (Codable schema, `schemaVersion`), `IOKitHelpers.swift`, `Collect/{Snapshot,USB,TypeC,Power,Displays}.swift`, `Render/{Text,Frame,Merge}.swift`, `TUI/{Terminal,Dashboard}.swift` | repo |
| Hooks | `.githooks/pre-commit` (runs `plugged-dev check`), `.githooks/commit-msg` (denylist grep) | repo |
| Denylist | `~/.local/share/plugged/denylist.txt` | ldf |
| This skill | `~/.config/plugged/skills/ari-dotfile--submodule-plugged/`, linked by `/ari-dotfiles-skill-registry` | repo |
| Build products | `$XDG_CACHE_HOME/swift/pkg/plugged/` through the dotfiles' `swift-pkg` (`swift-cache status` lists it); `.build/` (gitignored) only on a machine without `swift-pkg` | local |

## Commands

| command | does |
|---|---|
| `plugged` / `plugged json` | one JSON object: `schemaVersion`, `generatedAt`, `host`, `ports[]`, `usb[]` (tree), `displays[]`, `power`; keys sorted |
| `plugged text` | the same data as text: ports, USB tree (a hub's two halves merged by `containerId`), displays, power |
| `plugged tui [--interval S] [-]` | live dashboard; `-` or a piped stdin renders one JSON document from stdin (text view when stdout is not a TTY) |
| `plugged-dev build` | release build: `swift-pkg --build` into the shared Swift cache, else `swift build -c release` |
| `plugged-dev check` | build, `json \| jq -e .`, `json \| tui -`, `text`, leak grep of tracked files, commit identity; ends in one `[OK]` line |

## Rules

- **One collector per subsystem, one call each**, in `Collect/`; the model in `Model.swift` is the contract. Bump `schemaVersion` when a field changes meaning or disappears; adding a field does not.
- **No subprocesses.** IOKit and AppKit only. `system_profiler` returns no USB devices on some macOS builds and `ioreg` parsing is what this tool replaces.
- **JSON is the interface; text and TUI render the same model.** The renderer must accept JSON from stdin so `plugged json | plugged tui -` works; the JSON keeps both halves of a hub, only the renderers merge them.
- **Terminal rules** (`TUI/Terminal.swift`): alternate screen and cursor restored on every exit path, frames as `ESC[H` + line `ESC[K` + `ESC[J`, width by `ioctl(TIOCGWINSZ)` re-read on SIGWINCH, 16 ANSI colors only, single-cell glyphs only. Test in a detached tmux session, never in the user's.
- **Public repo: files and messages.** No company hostnames, ids, ticket numbers or colleague names. `plugged-dev check` greps tracked files and `user.email`, `commit-msg` greps the message, all against the local tier's denylist; without it nothing is enforced, so review by eye.
- **Commits**: `/ari-dotfile--submodule` § Working in one (on `main`, explicit paths, `git -C ~/.config/plugged add <files>`). Run `plugged-dev check` first. Conventional commits: `feat`, `fix`, `docs`, `refactor`; scopes `collect`, `render`, `tui`, `bin`, `skills`, `docs`.
- **The push is the user's**: `/ari-dotfile--submodule` § Pushing.

## Hooks

| hook | fires on | runs | skip |
|---|---|---|---|
| `.githooks/pre-commit` | every commit | `plugged-dev check` | never |
| `.githooks/commit-msg` | every commit | denylist grep of the message | never |

`core.hooksPath=.githooks` is local config; a fresh checkout has none until
`dotfiles apply plugged` sets it and builds.

## Workflow

1. Edit under `Sources/`; `plugged-dev check` (about 3 s incremental, 15 s from clean).
2. Commit per **Rules**.
3. `/ari-dotfile--submodule` § Pushing and § Verify: the user's `dotfiles push` publishes the repo and bumps the pointer.

Sync a machine after `/ari-dotfiles` refreshed the checkout: `plugged-dev build`
(or just run `plugged`; the wrapper builds when stale). Fresh machine:
`dotfiles apply plugged`.

## When it fails

Never `--no-verify`. Never edit `denylist.txt` to make a check pass. Detached
HEAD, pathspec-in-submodule, `dotfiles push` log lines, pointer refusals:
`/ari-dotfile--submodule` § When it fails.

| symptom | cause | do |
|---|---|---|
| `swift: command not found` or `xcrun: error` | no Xcode command line tools | `xcode-select --install`; do not install a toolchain another way |
| `plugged-dev check`: `denylisted identifiers in tracked files` | a company identifier in the public tree | remove it; re-run |
| `plugged-dev check`: `commit identity matches the denylist` | work `user.email` in the repo | `git -C ~/.config/plugged config user.email <personal address>` |
| `plugged json` shows no USB devices | nothing attached, or a new controller class | `ioreg -p IOUSB` to compare; the collector walks `IOUSBHostDevice` and the `IOUSB` plane parents |
| `plugged tui` leaves the terminal odd | killed with a signal the handler does not cover | `reset`; then add the signal to `installSignalPipe` |
| garbage on the first TUI paint | a pointer to a temporary in `Terminal.write` | use `withUnsafeBytes`; the regression test is the `tui -` round-trip in `check` |

## Key locations

- `README.md` (commands, sample output, the JSON shape), `LICENSE` (MIT).
- The IOKit property names each collector reads are the `Collect/*.swift` files' only comments worth reading.
