from simulator.pacman import handle_pacman
from simulator.paru import handle_paru
from simulator.systemctl import handle_systemctl
from simulator.net import handle_ip

class Simulator:
    def __init__(self, state):
        self.state = state

    def run(self, cmd: str) -> str:
        cmd = cmd.strip()

        if cmd.startswith("pacman"):
            return handle_pacman(cmd, self.state)

        if cmd.startswith("paru"):
            return handle_paru(cmd, self.state)

        if cmd.startswith("systemctl"):
            return handle_systemctl(cmd, self.state)

        if cmd.startswith("ip"):
            return handle_ip(cmd, self.state)

        if cmd.startswith("launch "):
            pkg = cmd.split(" ", 1)[1].strip()
            if pkg in self.state.installed_packages:
                return f"launching {pkg}"
            return f"{pkg}: not installed"

        return f"{cmd}: command not found"
