def audio_broken(state):
    state.errors["pipewire_missing"] = True
    state.services.pop("pipewire", None)

def pacman_broken(state):
    state.errors["pacman_broken"] = True

def network_down(state):
    state.errors["network_down"] = True
    for k in state.network:
        state.network[k] = "DOWN"
