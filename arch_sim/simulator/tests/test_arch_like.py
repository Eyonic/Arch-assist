import pytest

from simulator.state import SystemState
from simulator.runner import Simulator
from simulator.ai_runner import AIRunner
from simulator.scenarios import pacman_broken, network_down
from simulator.safety import validate


def test_pacman_q_lists_sorted_packages():
    state = SystemState()
    sim = Simulator(state)

    out = sim.run("pacman -Qq")
    assert out.splitlines() == sorted(state.installed_packages)


def test_pacman_system_upgrade_message():
    state = SystemState()
    sim = Simulator(state)

    out = sim.run("pacman -Syu")
    assert "Starting full system upgrade" in out


def test_pacman_remove_uninstalls_package():
    state = SystemState()
    sim = Simulator(state)
    state.installed_packages.add("vim")

    out = sim.run("pacman -R vim")
    assert "removing vim" in out
    assert "vim" not in state.installed_packages


def test_pacman_broken_surfaces_error():
    state = SystemState()
    pacman_broken(state)
    sim = Simulator(state)

    out = sim.run("pacman -S nano")
    assert "error while loading shared libraries" in out


def test_paru_q_lists_only_bin_packages():
    state = SystemState()
    state.installed_packages.update({"foo-bin", "bar"})
    sim = Simulator(state)

    out = sim.run("paru -Qq")
    assert out.splitlines() == ["foo-bin"]


def test_systemctl_user_status_for_running_service():
    state = SystemState()
    sim = Simulator(state)

    out = sim.run("systemctl --user status pipewire")
    assert "pipewire.service - running" == out


def test_systemctl_user_status_missing_service():
    state = SystemState()
    sim = Simulator(state)

    out = sim.run("systemctl --user status missing-svc")
    assert "Unit missing-svc.service could not be found" == out


def test_ip_link_after_network_down_scenario():
    state = SystemState()
    network_down(state)
    sim = Simulator(state)

    out = sim.run("ip link")
    assert all(line.endswith("DOWN") for line in out.splitlines())


def test_ai_delete_alias_returns_noop_when_missing():
    state = SystemState()
    sim = Simulator(state)
    ai = AIRunner(sim, state)

    cmd, output = ai.run("delete chrome")
    assert cmd == "true"
    assert output == ""


def test_ai_fix_sound_reports_status_when_healthy():
    state = SystemState()
    sim = Simulator(state)
    ai = AIRunner(sim, state)

    cmd, out = ai.run("fix sound")
    assert cmd == "systemctl --user status pipewire"
    assert "running" in out


def test_ai_open_existing_launches_without_reinstall():
    state = SystemState()
    state.installed_packages.add("firefox")
    sim = Simulator(state)
    ai = AIRunner(sim, state)

    cmd, out = ai.run("open firefox")
    assert cmd == "launch firefox"
    assert out == "launching firefox"


def test_safety_blocks_dangerous_tokens():
    with pytest.raises(ValueError):
        validate("pacman -Syu | rm -rf /")


def test_safety_allows_known_prefixes():
    # Should not raise
    validate("pacman -S vim")
