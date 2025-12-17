class SystemState:
    def __init__(self):
        self.installed_packages = {
            "bash", "linux", "pacman", "systemd", "paru", "networkmanager"
        }

        self.services = {
            "pipewire": "running",
            "NetworkManager": "running",
        }

        self.network = {
            "lo": "UP",
            "wlp2s0": "DOWN",
        }

        
        self.errors = {
            "pacman_broken": False,
            "pipewire_missing": False,
            "network_down": False,
        }
