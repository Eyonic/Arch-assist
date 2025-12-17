from simulator.state import SystemState
from simulator.runner import Simulator
from simulator.scenarios import audio_broken

def test_audio_broken():
    state = SystemState()
    audio_broken(state)
    sim = Simulator(state)

    out = sim.run("systemctl --user status pipewire")
    assert "could not be found" in out
