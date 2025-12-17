from simulator.state import SystemState
from simulator.runner import Simulator

def test_install_package():
    state = SystemState()
    sim = Simulator(state)

    sim.run("pacman -S firefox")
    assert "firefox" in state.installed_packages
