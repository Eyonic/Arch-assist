from simulator.state import SystemState
from simulator.runner import Simulator

def test_install_aur_package():
    state = SystemState()
    sim = Simulator(state)

    sim.run("paru -S brave-bin")
    assert "brave-bin" in state.installed_packages
