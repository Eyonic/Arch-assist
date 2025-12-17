from simulator.ai_openai import translate as llm_translate
from simulator.safety import validate

def _builtin_translate(user_input: str, state):
    """
    Lightweight, state-aware translator to avoid LLM calls for common actions.
    Returns a command string or None to fall back to the LLM.
    """
    text = user_input.strip()
    lower = text.lower()

    if lower.startswith("install "):
        pkg = text.split(None, 1)[1].strip()
        if not pkg:
            return None

        if pkg in state.installed_packages:
            return "true"

        installer = "paru" if pkg.endswith("-bin") else "pacman"
        return f"{installer} -S {pkg}"

    if lower.startswith(("remove ", "uninstall ", "delete ")):
        pkg = text.split(None, 1)[1].strip()
        if not pkg:
            return None

        if pkg not in state.installed_packages:
            return "true"

        installer = "paru" if pkg.endswith("-bin") else "pacman"
        return f"{installer} -R {pkg}"

    if "sound" in lower or "audio" in lower:
        needs_pipewire = (
            state.errors.get("pipewire_missing")
            or "pipewire" not in state.installed_packages
            or "pipewire" not in state.services
        )
        if needs_pipewire:
            return "pacman -S pipewire"
        return "systemctl --user status pipewire"

    if "internet" in lower or "network" in lower:
        network_down = state.errors.get("network_down") or any(
            status == "DOWN" for status in state.network.values()
        )
        if network_down:
            return "systemctl restart NetworkManager"
        return "ip link"

    if lower.startswith("open ") or lower.startswith("launch ") or lower.startswith("start "):
        pkg = text.split(None, 1)[1].strip()
        if not pkg:
            return None

        if pkg in state.installed_packages:
            return f"launch {pkg}"

        installer = "paru" if pkg.endswith("-bin") else "pacman"
        return f"{installer} -S {pkg}"

    return None

class AIRunner:
    def __init__(self, simulator, state):
        self.sim = simulator
        self.state = state

    def run(self, user_input):
        lowered = user_input.strip().lower()

        # handle open/launch/start intents with auto-install then launch
        if lowered.startswith(("open ", "launch ", "start ")):
            pkg = user_input.split(None, 1)[1].strip() if " " in user_input else ""
            if not pkg:
                return "true", ""

            install_output = ""
            if pkg not in self.state.installed_packages:
                installer = "paru" if pkg.endswith("-bin") else "pacman"
                install_cmd = f"{installer} -S {pkg}"
                validate(install_cmd)
                install_output = self.sim.run(install_cmd)

            launch_cmd = f"launch {pkg}"
            validate(launch_cmd)
            launch_output = self.sim.run(launch_cmd)

            combined_output = "\n".join(
                part for part in (install_output, launch_output) if part
            )
            return launch_cmd, combined_output

        cmd = _builtin_translate(user_input, self.state)
        if cmd is None:
            cmd = llm_translate(user_input, self.state)

        # No-op: nothing to do
        if cmd == "true":
            return "true", ""

        validate(cmd)
        output = self.sim.run(cmd)
        return cmd, output
