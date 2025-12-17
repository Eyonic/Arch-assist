def handle_ip(cmd, state):
    if cmd == "ip link":
        lines = []
        for iface, status in state.network.items():
            lines.append(f"{iface}: {status}")
        return "\n".join(lines)

    return "ip: unknown command"
