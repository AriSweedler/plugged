---
name: ari-dotfile--submodule-plugged
description: Work on `plugged`, the Rust CLI and dashboard that reports what is plugged into the machine it runs on (ports, USB tree, displays, power), from IOKit and AppKit on macOS and sysfs on Linux, with no subprocesses; docs/SPEC.md is its contract and tests/ checks it, synthetic Linux sysfs trees included. The repo is a dotfiles dev repo, a plain clone at ~/dotfiles/.config/plugged (linked to ~/.config/plugged) that the shared Nix flake reads as an input and builds onto PATH on every machine, Linux included; this skill has its commands, layout, rules, workflow (the co2 rollout included) and failures. Commits, pushes and the flake.lock bump follow ari-dotfile--submodule.
---

# plugged in the dotfiles

`plugged` is one Rust crate that shows what is plugged into the machine it runs
on: bare `plugged` is the live dashboard, `plugged --json` prints one snapshot,
`plugged --text` renders it. On macOS it reads the IORegistry through IOKit and
the screens through AppKit; on Linux it reads sysfs; it never spawns a process.
`docs/SPEC.md` is the contract every platform fills, and `tests/` checks it. The
repo is a dotfiles *dev repo*, not a git submodule: a plain clone the shared
dotfiles repo ignores and the shared Nix flake reads as an input.
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
| Package | `nix/pkgs/plugged`: `rustPlatform.buildRustPackage` with `cargoLock.lockFile` from the repo's `Cargo.lock`; `nix/home.nix` installs `ari.plugged` on every machine, Linux included | df |
| Binary on PATH | Nix's, in the user's Nix profile (`command -v plugged`); on a Mac `/etc/profiles/per-user/$USER/bin/plugged` | df |
| Bootstrap step | `~/dotfiles/.config/dotfiles/steps/plugged.zsh`: checks the commit identity, `core.hooksPath=.githooks`, and that Nix's `plugged` is installed; its apply sets identity and hooks and builds nothing | df |
| Dev wrapper | `~/.config/plugged/bin/plugged`: builds when `Cargo.toml`, `Cargo.lock` or a `src/**/*.rs` is newer than the binary, then `exec`s it; not on PATH | repo |
| Dev driver | `~/.config/plugged/bin/plugged-dev` (`build`, `test [--update]`, `check`) | repo |
| Crate | `Cargo.toml`, `Cargo.lock` (committed; crates `libc`, `serde`, `serde_json`); `src/main.rs` (arguments), `src/model.rs` (the snapshot structs, `SCHEMA_VERSION`) | repo |
| Collectors | `src/collect/mod.rs` picks the platform; macOS `src/collect/mac/{ioreg,ports,usb,power,displays}.rs`, with IOKit, CoreFoundation, CoreGraphics and AppKit declared by hand in `ioreg.rs` and `displays.rs`; Linux `src/collect/{sysfs,sysfs_usb,sysfs_power,sysfs_displays,sysfs_net,sysfs_ports,sysfs_smc}.rs` (`sysfs_smc.rs` reads an Intel Mac's SMC through applesmc's `key_at_index` files: `AC-N`, `AC-W`, `DnVR`, `DnIR`, `Dnin`) | repo |
| Renderers, TUI | `src/render/text.rs` (the text view); `src/render/frame.rs` (the dashboard: one box of stacked panels, Ports, Power, Devices, Network, Displays, built as a grid of styled `Cell`s per `Row`, with each `Link` drawn down the gutter's one rail, emitted by `render_ansi` or `render_plain`); `src/render/mod.rs` (`feeding_port`, `linked_interface`, and a device's `label`, `detail` and `size`, shared by both views: a disk goes by its volumes); `src/render/merge.rs`; `src/tui/{terminal,dashboard}.rs` | repo |
| Spec and tests | `docs/SPEC.md`; the unit tests in `src/render/frame.rs`'s `#[cfg(test)]` module, for rendering no snapshot reaches (two links that cross); `tests/conform.jq` (the spec's checkable rules); `tests/fixtures/<name>/sys`, a synthetic sysfs tree (the *fixture*), with `expected.json` (`generatedAt` and `host` pinned), `expected.txt` and `expected.frame.txt` beside it; `tests/make-fixtures.py` writes every fixture, one section per machine (`laptop-nixos`, whose `class/net` has Wi-Fi, a built-in Ethernet port with no cable and the virtual interfaces that are skipped; `macbook-dock`, the same fed through a dock, with readable SMC keys as files under `devices/platform/applesmc.768/keys/<NAME>`, a USB disk and the dock's Ethernet adapter, which draws the network link); `tests/snapshots/<name>.json`, a synthetic snapshot document from any platform, with `<name>.txt` and `<name>.frame.txt` beside it (`mac-charging` draws the power link and reports no interfaces) | repo |
| Hooks | `.githooks/pre-commit` (runs `plugged-dev check`), `.githooks/commit-msg` (denylist grep of the message); `core.hooksPath` is local config | repo |
| Docs | `README.md` (commands, sample output, the JSON shape, each platform's data sources), `LICENSE` (MIT) | repo |
| Denylist | `~/.local/share/plugged/denylist.txt` | ldf |
| This skill | `~/.config/plugged/skills/ari-dotfile--submodule-plugged/`, linked into `~/.claude/skills` by the shared flake (`nix/claude-skills.nix`) | repo |
| Dev build products | `$XDG_CACHE_HOME/plugged/target/`, never in the checkout | untracked (this machine) |

## Commands

| command | does |
|---|---|
| `plugged [--interval SECONDS]` | live dashboard, every 2 s by default; `q` or Ctrl-C quits; when stdin or stdout is not a terminal, the text view once; an interval of 0 or over 86400 exits 2 |
| `plugged --json` | one JSON object: `schemaVersion`, `generatedAt`, `host`, `ports[]`, `usb[]` (tree; a disk with optional `storage`: `bytes`, `volumes`), `displays[]`, `power` (with optional `adapter` and `battery`); keys sorted; `json` is the same |
| `plugged --text` | the same data as text: ports, USB tree (a hub's two halves merged by `containerId`), displays, power and the port it comes in on; `text` is the same |
| `plugged tui -` | renders one JSON document from stdin, from this or any machine, and exits (text view when stdout is not a TTY); a piped stdin implies `-`; a document that is not JSON or of another `schemaVersion` exits 1; `plugged tui` on a terminal is the dashboard |
| `plugged tui - --width COLUMNS` | one dashboard frame that wide, uncolored when stdout is not a TTY; the tests compare it at 84 columns |
| `~/.config/plugged/bin/plugged <args>` | the same commands from the working tree, uncommitted edits included |
| `PLUGGED_SYSFS_ROOT=<dir> ~/.config/plugged/bin/plugged --json` | the Linux collectors over a sysfs tree, on any OS (`<dir>` is a fixture's `sys`); a missing `<dir>` prints an empty snapshot and exits 0 |
| `plugged-dev build` | `cargo clippy` with warnings as errors, then `cargo build --release --locked`, into `$XDG_CACHE_HOME/plugged/target`; cargo from PATH, else `nix shell nixpkgs#cargo nixpkgs#rustc nixpkgs#clippy --command cargo` for the build |
| `plugged-dev test` | build, then `cargo test` (the renderers' own tests), then this machine's snapshot against `tests/conform.jq`; every fixture against `conform.jq` and its three expected files; every `tests/snapshots/` document against `conform.jq`, its text and its frame; the exit codes of bad arguments and bad documents (`expect_exit`) |
| `plugged-dev test --update` | rewrites every expected file, fixtures' and snapshots', from the current build |
| `plugged-dev check` | build, `bin/plugged json \| jq -e .`, `json \| tui -`, `text`, the tests, leak grep of tracked files, commit identity; ends in one `[OK]` line |
| `dotfiles switch` | builds the checkout's HEAD with Nix and puts it on PATH (`/ari-dotfiles-nix` § Switch) |
| `dotfiles healthcheck --only plugged` | the `plugged` step's verdict, read-only: `ok plugged built hooks='.githooks' binary=…` |

## Rules

- **The spec is the contract. Any edit to `src/model.rs` MUST change `docs/SPEC.md` in the same commit**, and `tests/conform.jq` when it adds a checkable rule. A platform's unknown is the value in the spec's Unknown column, never a guess and never `null` unless the column says so: on Linux `ports: []` (on an Intel Mac whose SMC can be read, its power ports only: `active` when a source is on it, no `transports`), on macOS `network: []` until the registry's interface side is read, a disk's `volumes: []`, `main: false`, `hz: 0` when the EDID gives no rate, and no `containerId` or `adapter` key (both optional; `adapter` only when a supply publishes its contract or the SMC is read). A root device's `port` is a port `id` or `bus N`. `power.battery` (watts, volts, amps into the battery while charging, out of it otherwise) is optional too, left out when nothing flows.
- **Raise the schema version when a field changes meaning or disappears** (a renamed key, a renumbered `speed`, a field turned optional): `SCHEMA_VERSION` in `src/model.rs`, the `schemaVersion` row of `docs/SPEC.md` and `must(.schemaVersion == …)` in `tests/conform.jq`, then `plugged-dev test --update`. Adding an optional field raises nothing.
- **One collector per subsystem per platform.** Each collector names in its code the IOKit properties or sysfs files it reads. To add a platform, add its collectors, a branch in `src/collect/mod.rs`, a row in the spec's Platforms table, and tests.
- **No subprocesses in the binary, on any platform. NEVER use `std::process::Command` or `fork`/`exec` in `src/`:** read a new field through hand-declared FFI on macOS (in `ioreg.rs` or `displays.rs`) or from a file under `/sys` on Linux. `grep -rn 'process::Command' ~/.config/plugged/src` MUST print nothing. `system_profiler` returns no USB devices on some macOS builds, and `ioreg` parsing is what this tool replaces; running either in a shell to compare output is fine.
- **JSON is the interface; text and the dashboard render the same model.** `plugged --json | plugged tui -` must keep working; the JSON keeps both halves of a hub, only `src/render/merge.rs` merges them. Both views find the port that feeds the machine with `feeding_port` (the first port whose `powerIn` is `true`): the dashboard draws a yellow link from its row to the Power panel's source line, and a platform with no ports shows `port unknown`. The same way, `linked_interface` is where the network comes in (the first `connected` interface with a `usbLocationId`), and the dashboard draws a blue link from that device's row in Devices to its row in Network; a platform that reports no interfaces has no Network panel.
- **The dashboard uses the 16 named ANSI colors plus bold and dim, never RGB.** Power is yellow everywhere: the port, the link, the panel. The network is blue the same way: the device it comes in on, the link, the panel.
- **Test data is synthetic. NEVER copy files from a real `/sys`, `ioreg` output or a real `plugged --json` into `tests/`:** the repo is public. A fixture is a section of `tests/make-fixtures.py`: edit a machine's section, or add one for a new machine (as `macbook-dock` starts from `laptop-nixos` and writes what differs). The SMC stands in as files under `devices/platform/applesmc.768/keys/<NAME>`. Invent every serial, EDID, SMC value, `containerId` and host name (as `SN00000001` and `host: "mac"` in `tests/snapshots/mac-charging.json`), run `python3 tests/make-fixtures.py` for a fixture, then `plugged-dev test --update`, and commit the script or document, `sys/` and the expected files together.
- **A change to either view is a diff in the expected files, reviewed on purpose. After `plugged-dev test --update`, read `git -C ~/.config/plugged diff -- tests/` before committing.** Every changed line, in `expected.txt`, `expected.frame.txt` or a snapshot's `.txt` and `.frame.txt`, must be one this change meant; else `git -C ~/.config/plugged checkout -- tests/` and fix the code. Name the files that changed, and why, in the commit report.
- **Cargo.lock is committed.** Nix builds from it (`cargoLock.lockFile`), so a dependency change is a `Cargo.lock` change in the same commit.
- **Clippy is fixed in the code. NEVER drop `-D warnings`, and NEVER add `#[allow(...)]`**, except for the Linux dead-code case below.
- **On a Mac, run a Linux clippy before every commit that touches `src/`, `Cargo.toml` or `Cargo.lock`.** An item only the macOS collectors use is dead code on Linux, so Linux clippy fails while the Mac `[OK]` passes. Gate it with `#[cfg(target_os = "macos")]`, or `#[cfg_attr(not(target_os = "macos"), allow(dead_code))]` when both platforms compile it (as in `src/model.rs`); never a bare `#[allow(dead_code)]`. The copy lives under the home directory, which colima mounts:
  ```zsh
  colima status || colima start
  d=~/.cache/plugged/linux-src; mkdir -p $d
  rsync -a --delete ~/.config/plugged/{Cargo.toml,Cargo.lock,src,tests} $d/
  docker run --rm --platform linux/amd64 -v $d:/src -w /src rust:1 cargo clippy --release -- -D warnings
  ```
  On Linux, `plugged-dev build` is this check. When docker cannot run it, say in the commit report that the Linux clippy did not run.
- **TUI work follows `/ari-tui-dashboard`**: its tmux rules (alternate screen, frame, SIGWINCH, colors, glyphs) and its **Test recipe**, in a detached tmux session it creates and kills, never the user's; the live loop there is bare `~/.config/plugged/bin/plugged`. From Claude's Bash, bare `plugged` prints the text view once and `plugged tui` reads stdin and fails with `stdin is not a plugged JSON document`; both are expected, not regressions.
- **`plugged` on PATH is a commit, never the working tree. Test every edit with `~/.config/plugged/bin/plugged`, never bare `plugged`.** `dotfiles switch` overrides the flake's `plugged` input with the checkout's HEAD, so a local commit, pushed or not, is on PATH after a switch. Without a checkout, Nix builds the locked commit. `jq -r .nodes.plugged.locked.rev ~/.local/share/dotfiles/nix/flake.lock` is the commit the last switch locked, written before it built: equal to `git -C ~/.config/plugged rev-parse HEAD` does not mean the build landed; read the last switch's output (`/ari-dotfile--submodule` § How the build and the lock follow the checkout).
- **Public repo: read the check's output, not just its `[OK]`.** `plugged-dev check` greps tracked files and `user.email`, and `commit-msg` greps the message, all against the local tier's denylist. **When `check` logs `leak grep skipped: no denylist` (no `leak grep clean` and no `commit identity ok` above its `[OK]`), neither the check nor `commit-msg` checked anything:** read `git -C ~/.config/plugged diff --cached`, the message and `git -C ~/.config/plugged config user.email` for company identifiers yourself, and tell the user the denylist is missing.
- **Commits** per `/ari-dotfile--submodule` § Working in one: `plugged-dev check` first, **NEVER `--no-verify`**. The hook checks the working tree, not the index, and the build compiles untracked `src/` files the commit leaves out. **Before `git commit`, run `git -C ~/.config/plugged status --short`:** any unstaged line, or `??` under `src/`, `tests/` or `docs/`, means `git -C ~/.config/plugged stash push --keep-index --include-untracked` first and `git -C ~/.config/plugged stash pop` after the commit, whether it landed or not. A pop that conflicts: stop, name the stash to the user, never drop it. Types `feat`, `fix`, `docs`, `refactor`, `build`; scopes `collect`, `render`, `tui`, `bin`, `skills`, `docs`; e.g. `fix(collect): read the TypeC port's current power role`.
- **NEVER push.** The push and the lock bump are the user's (`/ari-dotfile--submodule` § Pushing): `dotfiles push` pushes the repo, then runs `nix flake update plugged` and commits `flake.lock` alone in the shared tier, e.g. `plugged: bump to bf0421f (docs(skills): new-machine apply is dotfiles apply)`.

## Workflow

### Change plugged

A change only under `skills/`, `README.md` or `docs/` that leaves `docs/SPEC.md` alone needs steps 1, 3 and 5 only; the skill is live once saved.

1. Edit under `src/`, with `docs/SPEC.md` and `tests/` per **Rules**; `plugged-dev check`. A changed view or Linux collector fails an expected file when it changes the output: `plugged-dev test --update`, then read its diff per **Rules**.
2. The diff touches `src/`, `Cargo.toml` or `Cargo.lock`: Linux clippy per **Rules**.
3. Commit per **Rules**.
4. Switch per `/ari-dotfiles-nix` § Switch (Claude runs it, `--build` first) when the user asked to run the change as `plugged`, or the task ends with a behavior change committed; otherwise end by saying `plugged` on PATH is still the old commit.
5. Say "Run `dotfiles push` when ready." and stop: the push and the lock bump are the user's (`/ari-dotfile--submodule` § Pushing).

### Roll out to co2

co2, the NixOS homelab machine, builds `plugged` from the dotfiles flake it takes as an input. Run this only when the user asks for it, and only after he says he pushed the lock bump. Reach co2 per `/ari-homelab`.

1. In the homelab's `hosts/co2`: `nix flake update dotfiles`. The `plugged` rev in its `flake.lock` unchanged → stop: the bump is not upstream yet, nothing new for co2.
2. Commit; the push is the user's.
3. `hosts/co2/bin/co2-remote install` (a dry run); show the user its output.
4. Only on his yes: `hosts/co2/bin/co2-remote install --apply`.
5. co2's SMC power data needs `modules/smc.nix` (a udev rule that opens `key_at_index` to every user). After the switch that first adds it, the rule runs only on the device's next add event: replay it once with `sudo udevadm trigger --action=add /sys/devices/platform/applesmc.768`, which the user runs or allows. Without it `plugged` on co2 reports no ports and no adapter, as before.

### Set up a machine

1. `dotfiles apply dotfiles_repo` clones the checkout.
2. `dotfiles apply plugged` sets the personal commit identity and `core.hooksPath=.githooks`, which a fresh clone lacks; it builds nothing.
3. `dotfiles switch` installs the binary.

After a pull moves the checkout, switch again.

## When it fails

Generic rules and rows (detached HEAD, `--no-verify`, `dotfiles push` log
lines, lock-bump refusals): `/ari-dotfile--submodule` § When it fails. The
`plugged` step's rows come from `dotfiles healthcheck --only plugged`. **A
`skip` exits 0: report the step healthy only when its line starts `ok plugged`.**

| symptom | cause | do |
|---|---|---|
| healthcheck `skip plugged no_submodule`: `plugged not checked out` | no clone at `~/.config/plugged` | `dotfiles apply dotfiles_repo` |
| healthcheck `identity_drift` or `hooks_unwired` | a fresh clone's local config | `dotfiles apply plugged` |
| healthcheck `not_built`: `Nix's plugged is not installed` | no switch since the package was added | `dotfiles switch` |
| `plugged` on PATH lacks a change | the change is uncommitted, committed after the last switch, or the last switch's build failed (**Rules**: on PATH is a commit) | commit, then `dotfiles switch`; meanwhile `~/.config/plugged/bin/plugged` |
| `plugged-dev`: `no cargo on PATH and no nix to fetch one` | neither toolchain source | install Nix per `/ari-dotfiles-nix`; `plugged-dev` then fetches cargo itself. NEVER `brew install rust`, `rustup` or a `curl … \| sh` installer |
| `plugged-dev build` fails with a Nix download error | no cargo on PATH and no network for `nix shell` | wait for the network; never commit around the hook |
| `plugged-dev build` fails in `cargo clippy` | a clippy warning, which `-D warnings` makes an error; on code this change did not touch, a newer clippy from a moved `nixpkgs` | fix it per **Rules**; an untouched file's lint in its own `fix` commit |
| a Linux clippy fails while the Mac build passes | a macOS-only item, dead on Linux | gate it with `cfg` per **Rules** |
| `colima start` or the `rust:1` pull fails | no container runtime, or no network | say in the commit report that the Linux clippy did not run |
| `dotfiles switch --build` fails on `Cargo.lock` while `plugged-dev check` passes | `Cargo.toml` changed and the `Cargo.lock` cargo rewrote in the working tree was not committed | commit `Cargo.lock` |
| `dotfiles switch --build` fails on a missing module while `plugged-dev check` passed | the file was untracked at the commit | `git -C ~/.config/plugged add` it and commit again |
| `plugged-dev` ends after an `[INFO]` line with no `[ERROR]` and no `[OK]` | the stage that line names failed: the build, a collector panic, `tui -` unable to decode the JSON, or `text` | run that stage by hand, e.g. `~/.config/plugged/bin/plugged --json \| ~/.config/plugged/bin/plugged tui - >/dev/null`; a decode error means `src/model.rs` and the JSON disagree on a field, or the document's `schemaVersion` is not `SCHEMA_VERSION` |
| `plugged-dev test`: `cargo test failed` | a renderer unit test in `src/render/frame.rs` failed; cargo's output above the line names it | fix the code, or the test when the drawing changed on purpose |
| every spec test fails at once, or expected files are empty after `--update` | no `jq` (only `check` tests for it) | install `jq` per `/ari-dotfiles-nix`, then `git -C ~/.config/plugged checkout -- tests/` |
| a log line ending `json breaks the spec` | a snapshot breaks a rule in `tests/conform.jq` | `~/.config/plugged/bin/plugged --json \| jq -e -f ~/.config/plugged/tests/conform.jq` names it (`spec: <rule>`); fix the collector, or the spec and `conform.jq` together |
| a log line `… differs from expected…` | a collector or renderer changed a fixture's or snapshot's output (json, text or dashboard frame); the `diff -u` above the line shows how | intended → `plugged-dev test --update` and read its diff per **Rules**; else fix the code |
| a fixture's expected files list no devices | its `sys/` is missing | `python3 tests/make-fixtures.py`, then `plugged-dev test --update` |
| `plugged-dev test`: an `expect_exit` case fails | a bad interval or a bad document no longer exits 2 or 1 | fix the argument or document check in `src/main.rs` or `src/tui/dashboard.rs` |
| Linux on an Intel Mac shows no ports and no adapter | the user cannot write applesmc's `key_at_index` (root only by default) | on co2, `modules/smc.nix` and its one-time `udevadm trigger` (**Roll out to co2**); elsewhere expected |
| `plugged-dev check`: `denylisted identifiers in tracked files` | a company identifier in the public tree | remove it; re-run |
| `plugged-dev check`: `commit identity matches the denylist` | work `user.email` in the repo | `dotfiles apply plugged` |
| `plugged --json` shows no USB devices on a Mac | nothing attached, or a new controller class | `ioreg -p IOUSB` to compare; `src/collect/mac/usb.rs` walks `IOUSBHostDevice` and its `IOUSB` plane parents |
| `plugged --json` shows no USB devices on Linux | nothing attached, or names under `/sys/bus/usb/devices` that are not `<bus>-<port>.<port>…` | `ls /sys/bus/usb/devices` to compare with `src/collect/sysfs_usb.rs` |
| the dashboard leaves the terminal odd | killed with a signal the handler does not cover | kill the test session (`/ari-tui-dashboard` § Test recipe), or the user runs `reset` in his own terminal; a catchable signal goes into the `install_signal_pipe` call in `src/tui/dashboard.rs` (SIGKILL cannot be caught: `reset` only) |
