# Arch Assist (Rust)

Minimal Arch helper that translates common intent (install/remove/open/fix) into real system commands.

## Install
- Ensure Rust toolchain: `sudo pacman -S rustup` then `rustup default stable`
- From repo root: `cd rust && cargo install --path .`
- Binary will be at `~/.cargo/bin/arch-assist`

## Usage
- Suggest commands only (default): `arch-assist ai "open brave"` (prints commands; does not run)
- Apply suggestions: `arch-assist --apply ai "open brave"`
- Prefer paru over pacman: `arch-assist --prefer-paru ai "install firefox"`
- Avoid sudo for pacman: `arch-assist --no-sudo ai "install base"`
- Verbose exit codes: `arch-assist --verbose --apply ai "fix internet"`
- Install package suggestion: `arch-assist ai "install firefox"`
- Remove package suggestion (alias: uninstall/delete): `arch-assist ai "uninstall firefox"`
- Open app suggestion (auto-install+launch if applied): `arch-assist --apply ai "open vlc"`
- Fix sound/internet suggestions: `arch-assist ai "fix sound"` / `arch-assist ai "fix internet"`

Commands run directly on your system (pacman/paru/systemctl). Keep `--dry-run` on if you just want the suggested commands.
