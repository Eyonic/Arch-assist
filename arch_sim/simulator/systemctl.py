def handle_systemctl(cmd, state):
    parts = cmd.split()

    if "pipewire" in cmd and state.errors.get("pipewire_missing"):
        return "Unit pipewire.service could not be found"

    if parts[:3] == ["systemctl", "--user", "status"]:
        svc = parts[-1]
        status = state.services.get(svc)
        if status:
            return f"{svc}.service - {status}"
        return f"Unit {svc}.service could not be found"

    return "systemctl: unknown command"
