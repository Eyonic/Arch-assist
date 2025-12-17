from simulator.state import SystemState
from simulator.runner import Simulator
from simulator.ai_runner import AIRunner


def test_ai_state_mutates_across_runs():
    state = SystemState()
    sim = Simulator(state)
    ai = AIRunner(sim, state)

    cmd, _ = ai.run("install brave")
    assert cmd == "pacman -S brave"
    assert "brave" in state.installed_packages

    cmd, _ = ai.run("remove brave")
    assert cmd == "pacman -R brave"
    assert "brave" not in state.installed_packages

    cmd, _ = ai.run("install brave-bin")
    assert cmd == "paru -S brave-bin"
    assert "brave-bin" in state.installed_packages


def test_ai_fix_sound_installs_pipewire_when_missing():
    state = SystemState()
    state.errors["pipewire_missing"] = True
    state.services.pop("pipewire", None)
    sim = Simulator(state)
    ai = AIRunner(sim, state)

    cmd, _ = ai.run("fix sound")
    assert cmd == "pacman -S pipewire"
    assert "pipewire" in state.installed_packages
    assert state.services.get("pipewire") == "running"
    assert state.errors["pipewire_missing"] is False


def test_ai_fix_internet_restarts_networkmanager():
    state = SystemState()
    for iface in state.network:
        state.network[iface] = "DOWN"
    state.errors["network_down"] = True

    sim = Simulator(state)
    ai = AIRunner(sim, state)

    cmd, _ = ai.run("fix internet")
    assert cmd == "systemctl restart NetworkManager"
    assert all(status == "UP" for status in state.network.values())
    assert state.errors["network_down"] is False


def test_ai_open_installs_then_launches():
    state = SystemState()
    sim = Simulator(state)
    ai = AIRunner(sim, state)

    # Not installed -> installs then launches
    cmd, out = ai.run("open brave")
    assert cmd == "launch brave"
    assert "installing brave" in out
    assert "launching brave" in out
    assert "brave" in state.installed_packages

    # Already installed -> launch
    cmd, out = ai.run("open brave")
    assert cmd == "launch brave"
    assert out == "launching brave"


def test_ai_delete_alias_removes_package():
    state = SystemState()
    sim = Simulator(state)
    ai = AIRunner(sim, state)

    ai.run("install chrome")
    assert "chrome" in state.installed_packages

    cmd, _ = ai.run("delete chrome")
    assert cmd == "pacman -R chrome"
    assert "chrome" not in state.installed_packages
