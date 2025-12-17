from simulator.state import SystemState
from simulator.runner import Simulator
from simulator.ai_runner import AIRunner

state = SystemState()
sim = Simulator(state)
ai = AIRunner(sim, state)




while True:
    raw = input("archsim $ ").strip()

    if raw in ("exit", "quit"):
        break

    # 🔥 AI path
    if raw.startswith("ai "):
        user_input = raw[3:]  # remove "ai "
        cmd, output = ai.run(user_input)

        # IMPORTANT: show rewritten command
        print(cmd)

        # For now, still execute in simulator
        print(output)
        continue

    # normal command path
    print(sim.run(raw))
