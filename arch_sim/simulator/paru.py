def handle_paru(cmd, state):
    parts = cmd.split()

    if parts == ["paru", "-Qq"]:
        return "\n".join(sorted(
            p for p in state.installed_packages if p.endswith("-bin")
        ))

    if parts[:2] == ["paru", "-S"]:
        pkg = parts[-1]
        state.installed_packages.add(pkg)
        return f":: Resolving AUR dependencies...\n:: Installing {pkg}"

    return "paru: invalid operation"
