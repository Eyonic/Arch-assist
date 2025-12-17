# Arch Assist (Rust)

Minimal Arch helper that translates common intent (install/remove/open/fix) into real system commands.

## Install
- Ensure Rust toolchain: `sudo pacman -S rustup` then `rustup default stable`
- From repo root: `cd rust && cargo install --path .`
- Binary will be at `~/.cargo/bin/arch-assist`

## Usage
- Suggest commands only (default): `arch-assist ai "open brave"` (prints commands; does not run)
- Auto-run suggestions: `arch-assist --auto ai "open brave"`
- Prefer paru over pacman: `arch-assist --prefer-paru ai "install firefox"`
- Avoid sudo for pacman: `arch-assist --no-sudo ai "install base"`
- Add --noconfirm to package ops: `arch-assist --yes --auto ai "install vlc"`
- Block package downloads (offline): `arch-assist --offline ai "upgrade system"` (will refuse)
- Verbose exit codes: `arch-assist --verbose --auto ai "fix internet"`
- Install package suggestion: `arch-assist ai "install firefox"`
- Remove package suggestion (alias: uninstall/delete): `arch-assist ai "uninstall firefox"`
- Open app suggestion (auto-install+launch if auto): `arch-assist --auto ai "open vlc"`
- Fix sound/internet suggestions: `arch-assist ai "fix sound"` / `arch-assist ai "fix internet"`
- Upgrade system: `arch-assist ai "upgrade system"`
- Clean cache: `arch-assist ai "clean cache"`
- Logs for a service: `arch-assist ai "logs sshd"`
- Bluetooth fix: `arch-assist ai "fix bluetooth"`
- Time sync fix: `arch-assist ai "fix time"`
- Quick AI smoke test: `arch-assist --offline ai "test ai"` (prints built-in or LLM fallback; use `--offline` to avoid network)

Commands run directly on your system (pacman/paru/systemctl). Keep `--dry-run` on if you just want the suggested commands.
When `--auto` is used, you'll be asked to confirm unless `--yes` is provided.

## OpenAI
- Set `OPENAI_API_KEY=sk-...` in your environment to enable LLM fallbacks.
- Optional: override model with `OPENAI_MODEL` (default: `gpt-4o-mini`).
- Use `--offline` to force built-ins only and avoid network during testing.
