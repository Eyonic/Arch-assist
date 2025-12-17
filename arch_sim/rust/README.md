# Arch Assist (Rust)

Minimal Arch helper that translates common intent (install/remove/open/fix) into real system commands.

## Install
- Ensure Rust toolchain: `sudo pacman -S rustup` then `rustup default stable`
- From repo root: `cd rust && cargo install --path .`
- Binary will be at `~/.cargo/bin/arch-assist`

## Usage
- Dry run (only print): `arch-assist --dry-run ai "open brave"`
- Prefer paru over pacman: `arch-assist --prefer-paru ai "install firefox"`
- Avoid sudo for pacman: `arch-assist --no-sudo ai "install base"`
- Verbose exit codes: `arch-assist --verbose ai "fix internet"`
- Install package: `arch-assist ai "install firefox"`
- Remove package (alias: uninstall/delete): `arch-assist ai "uninstall firefox"`
- Open app (auto-install if missing): `arch-assist ai "open vlc"`
- Fix sound: `arch-assist ai "fix sound"`
- Fix internet: `arch-assist ai "fix internet"`

Commands run directly on your system (pacman/paru/systemctl). Keep `--dry-run` on if you just want the suggested commands.
