from simulator.state import SystemState
from simulator.runner import Simulator

state = SystemState()
sim = Simulator(state)

while True:
    cmd = input("archsim $ ")
    if cmd in ("exit", "quit"):
        break
    print(sim.run(cmd))
