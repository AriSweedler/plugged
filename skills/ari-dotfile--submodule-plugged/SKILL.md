---
name: ari-dotfile--submodule-plugged
description: Work on `plugged`, the Swift CLI and TUI that reports what is plugged into this Mac (ports, USB tree, displays, power) from IOKit with no subprocesses. The repo is a dotfiles dev repo, a plain clone at ~/dotfiles/.config/plugged (linked to ~/.config/plugged) that the shared Nix flake reads as an input and builds onto PATH; this skill has its commands, layout, rules and failures. Commits, pushes and the flake.lock bump follow ari-dotfile--submodule.
---

# plugged in the dotfiles

`plugged` is one Swift package: `plugged json` prints one JSON document,
`plugged text` renders it, `plugged tui` renders it live. It reads the
IORegistry through IOKit and the display list through AppKit; it never spawns a
process. The repo is a dotfiles *dev repo*, not a git submodule: a plain clone
the shared dotfiles repo ignores and the shared Nix flake reads as an input.
`/ari-dotfile--submodule` has what every dev repo shares: the layout, `main`,
commits, the user's push and lock bump, the generic failures. This skill has
what is plugged-specific.

## Layout

*The checkout* is `~/dotfiles/.config/plugged/`, a plain clone on `main`, linked
whole to `~/.config/plugged`. There is no `.gitmodules`, no gitlink in the
shared repo, and no `git df submodule …` command to run.

| Piece | Path | Tier |
|---|---|---|
| Checkout (public) | upstream `github.com/AriSweedler/plugged`; df ignores it (`.gitignore`) and lists it in `~/dotfiles/.config/dotfiles/dev-repos` | repo |
| Flake input | `plugged` in `~/dotfiles/.config/dotfiles/nix/flake.nix` (github, `flake = false`), locked in `nix/flake.lock` | df |
| Binary on PATH | Nix's, `/etc/profiles/per-user/$USER/bin/plugged`, built by `nix/pkgs/plugged` from the input | df |
| Bootstrap step | `~/dotfiles/.config/dotfiles/steps/plugged.zsh`: checks the commit identity, `core.hooksPath=.githooks`, and that Nix's `plugged` is installed; its apply sets identity and hooks and builds nothing | df |
| Dev wrapper | `~/.config/plugged/bin/plugged`: builds `release` when a source is newer than the binary, then `exec`s it; not on PATH | repo |
| Dev driver | `~/.config/plugged/bin/plugged-dev` (`build`, `check`) | repo |
| Sources | `Sources/plugged/`: `main.swift`, `Model.swift` (Codable schema, `schemaVersion`), `IOKitHelpers.swift`, `Collect/{Snapshot,USB,TypeC,Power,Displays}.swift`, `Render/{Text,Frame,Merge}.swift`, `TUI/{Terminal,Dashboard}.swift` | repo |
| Hooks | `.githooks/pre-commit` (runs `plugged-dev check`), `.githooks/commit-msg` (denylist grep of the message); `core.hooksPath` is local config | repo |
| Docs | `README.md` (commands, sample output, the JSON shape), `LICENSE` (MIT) | repo |
| Denylist | `~/.local/share/plugged/denylist.txt` | ldf |
| This skill | `~/.config/plugged/skills/ari-dotfile--submodule-plugged/`, linked into `~/.claude/skills` by the shared flake (`nix/claude-skills.nix`) | repo |
| Dev build products | `$XDG_CACHE_HOME/swift/pkg/plugged/` through the dotfiles' `swift-pkg` (`swift-cache status` lists it); `.build/` (gitignored) only on a machine without `swift-pkg` | untracked (this machine) |

## Commands

| command | does |
|---|---|
| `plugged` / `plugged json` | one JSON object: `schemaVersion`, `generatedAt`, `host`, `ports[]`, `usb[]` (tree), `displays[]`, `power`; keys sorted |
| `plugged text` | the same data as text: ports, USB tree (a hub's two halves merged by `containerId`), displays, power |
| `plugged tui [--interval SECONDS] [-]` | live dashboard, every 2 s by default; `-` or a piped stdin renders one JSON document from stdin (text view when stdout is not a TTY) |
| `~/.config/plugged/bin/plugged <args>` | the same commands from the working tree, uncommitted edits included |
| `plugged-dev build` | release build of the working tree: `swift-pkg --build` into the shared Swift cache, else `swift build -c release` |
| `plugged-dev check` | build, `json \| jq -e .`, `json \| tui -`, `text`, leak grep of tracked files, commit identity; ends in one `[OK]` line |
| `dotfiles switch` | builds the checkout's HEAD with Nix and puts it on PATH (`/ari-dotfiles-nix` § Switch) |
| `dotfiles healthcheck --only plugged` | the `plugged` step's verdict, read-only: `ok plugged built hooks='.githooks' binary=…` |

## Rules

- **One collector per subsystem, one call each**, in `Collect/`; the model in `Model.swift` is the contract. The IOKit property names each collector reads are in the comments of `Collect/*.swift`. Bump `schemaVersion` when a field changes meaning or disappears; adding a field does not.
- **No subprocesses in the binary.** IOKit and AppKit only: `system_profiler` returns no USB devices on some macOS builds, and `ioreg` parsing is what this tool replaces. Running `ioreg` in a shell to compare output is fine.
- **JSON is the interface; text and TUI render the same model.** `plugged json | plugged tui -` must keep working; the JSON keeps both halves of a hub, only `Render/Merge.swift` merges them.
- **TUI work follows `/ari-tui-dashboard`**, whose worked example is `plugged tui`: its tmux rules (alternate screen, frame, SIGWINCH, colors, glyphs) and its **Test recipe**, in a detached tmux session it creates and kills, never the user's. `plugged tui` run straight from Claude's Bash reads stdin and fails with `stdin is not a plugged JSON document`; that is expected, not a regression.
- **`plugged` on PATH is a commit, never the working tree.** `dotfiles switch` overrides the flake's `plugged` input with the checkout's HEAD, so a local commit, pushed or not, is on PATH after a switch; without a checkout Nix builds the locked commit. An uncommitted edit runs only through `bin/plugged` and `plugged-dev`. The commit on PATH is `jq -r .nodes.plugged.locked.rev ~/.local/share/dotfiles/nix/flake.lock`; equal to `git -C ~/.config/plugged rev-parse HEAD` means there is nothing to switch.
- **Public repo: read the check's output, not just its `[OK]`.** `plugged-dev check` greps tracked files and `user.email`, and `commit-msg` greps the message, all against the local tier's denylist. **When `check` logs `leak grep skipped: no denylist`, its `[OK]` covers neither leaks nor identity:** read `git -C ~/.config/plugged diff --cached`, the message and `git -C ~/.config/plugged config user.email` for company identifiers yourself.
- **Commits** per `/ari-dotfile--submodule` § Working in one. Run `plugged-dev check` first. The hook checks the working tree, not the index: commit with nothing else unstaged, or `git -C ~/.config/plugged stash push --keep-index` first and `stash pop` after. Types `feat`, `fix`, `docs`, `refactor`, `build`; scopes `collect`, `render`, `tui`, `bin`, `skills`, `docs`; e.g. `fix(collect): read the TypeC port's current power role`.
- **The push is the user's** (`/ari-dotfile--submodule` § Pushing): `dotfiles push` pushes the repo, then runs `nix flake update plugged` and commits `flake.lock` alone in the shared tier, e.g. `plugged: bump to bf0421f (docs(skills): new-machine apply is dotfiles apply)`.

## Workflow

### Change plugged

1. Edit under `Sources/`; `plugged-dev check` (about 3 s incremental, 15 s from clean).
2. Commit per **Rules**.
3. When the commit should be the `plugged` on PATH, switch per `/ari-dotfiles-nix` § Switch: Claude runs it, building with `--build` first.
4. Pushing and the lock bump follow `/ari-dotfile--submodule` § Pushing.

### Set up a machine

1. `dotfiles apply dotfiles_repo` clones the checkout.
2. `dotfiles apply plugged` sets the personal commit identity and `core.hooksPath=.githooks`, which a fresh clone lacks; it builds nothing.
3. `dotfiles switch` installs the binary.

After a pull moves the checkout, switch again.

## When it fails

Generic rules and rows (detached HEAD, `--no-verify`, `dotfiles push` log
lines, lock-bump refusals): `/ari-dotfile--submodule` § When it fails. The
`plugged` step's rows come from `dotfiles healthcheck --only plugged`; a `skip`
exits 0, so read the reason word, not the exit code.

| symptom | cause | do |
|---|---|---|
| healthcheck `skip plugged no_submodule`: `plugged not checked out` | no clone at `~/.config/plugged` | `dotfiles apply dotfiles_repo` |
| healthcheck `identity_drift` or `hooks_unwired` | a fresh clone's local config | `dotfiles apply plugged` |
| healthcheck `not_built`: `Nix's plugged is not installed` | no switch since the package was added | `dotfiles switch` |
| `plugged` on PATH lacks a change | the change is uncommitted, or committed after the last switch (the **Rules** `jq` line tells which) | commit, then `dotfiles switch`; meanwhile `~/.config/plugged/bin/plugged` |
| `dotfiles switch --build` fails building `plugged` while `plugged-dev check` passes | Nix's Swift and SDK are older than Xcode's | gate the code with `#available` or `#if compiler(…)`, or drop the feature; fix before the push |
| `plugged-dev`: `swift: command not found` or `xcrun: error` | no Xcode command line tools for the dev build | the user runs `xcode-select --install` (a GUI installer); do not install a toolchain another way |
| `plugged-dev check` stops after `[INFO] plugged json \| …` with no `[ERROR]` | that stage failed: a collector crash, or `tui -` cannot decode the JSON | run the stage by hand, `~/.config/plugged/bin/plugged json \| jq -e .`; a `tui -` decode error means `Model.swift` and the JSON disagree (the `schemaVersion` rule) |
| `plugged-dev check`: `denylisted identifiers in tracked files` | a company identifier in the public tree | remove it; re-run |
| `plugged-dev check`: `commit identity matches the denylist` | work `user.email` in the repo | `dotfiles apply plugged` |
| `plugged json` shows no USB devices | nothing attached, or a new controller class | `ioreg -p IOUSB` to compare; the collector walks `IOUSBHostDevice` and the `IOUSB` plane parents |
| `plugged tui` leaves the terminal odd | killed with a signal the handler does not cover | kill the test session (`/ari-tui-dashboard` § Test recipe), or the user runs `reset` in his own terminal; then add the signal to `installSignalPipe` |
| garbage on the first TUI paint | a pointer to a temporary in `Terminal.write` | use `withUnsafeBytes`; the regression test is the `tui -` round-trip in `check` |
