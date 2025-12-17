def handle_pacman(cmd, state):
    if state.errors.get("pacman_broken"):
        return "pacman: error while loading shared libraries: libalpm.so.14"

    parts = cmd.split()

    if parts == ["pacman", "-Qq"]:
        return "\n".join(sorted(state.installed_packages))

    if parts[:2] == ["pacman", "-S"]:
        pkg = parts[-1]
        state.installed_packages.add(pkg)
        if pkg == "pipewire":
            state.errors["pipewire_missing"] = False
            state.services["pipewire"] = "running"
        return f"resolving dependencies...\ninstalling {pkg}"

    if parts[:2] == ["pacman", "-R"]:
        pkg = parts[-1]
        state.installed_packages.discard(pkg)
        return f"removing {pkg}"

    if parts == ["pacman", "-Syu"]:
        return ":: Synchronizing package databases...\n:: Starting full system upgrade..."

    return "pacman: invalid operation"
