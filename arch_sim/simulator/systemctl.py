def handle_systemctl(cmd, state):
    parts = cmd.split()

    if "pipewire" in cmd and state.errors.get("pipewire_missing"):
        return "Unit pipewire.service could not be found"

    if parts[:2] == ["systemctl", "restart"] and parts[-1] == "NetworkManager":
        state.errors["network_down"] = False
        state.services["NetworkManager"] = "running"
        for iface in state.network:
            state.network[iface] = "UP"
        return "Restarting NetworkManager"

    if parts[:3] == ["systemctl", "--user", "status"]:
        svc = parts[-1]
        status = state.services.get(svc)
        if status:
            return f"{svc}.service - {status}"
        return f"Unit {svc}.service could not be found"

    return "systemctl: unknown command"
