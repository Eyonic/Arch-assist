#AI-powered command-line assistant for Arch Linux that translates natural language into safe, transparent shell commands.

You type what you want in plain English (or troubleshooting descriptions), the AI rewrites your command in-place, and you decide whether to execute it by pressing Enter again. Nothing runs automatically.

#Key features:

🧠 Natural language → Arch Linux commands

🔁 Command-line rewriting (not auto-execution)

📦 Aware of installed packages (names only)

🔧 Troubleshooting mode (one safe step at a time)

🔐 Strong safety model (allowlists, no destructive commands)

🐚 Shell-native UX (bash/zsh/fish friendly)

🧱 Arch philosophy–compliant: explicit, transparent, user-controlled

Think of it as “AI autocomplete for your terminal”, not a replacement for pacman.


#Road-Map


1. The python Arch simulator is done :)
2. The rust code is working. (documentation is in the rust folder)
3. Testing on a clean arch linux (final boss)




#Plans
When smaller Ollama models become sufficiently capable, I want this system to run fully offline. For now, however, I’m using ChatGPT (specifically 4.1). I’d prefer to use ChatGPT-5, but the API is still a pain when I ask chatgtp 5 why it does not know even though it should, I end up going back to GPT-4o-mini instead.

Despite that, ChatGPT is still far more powerful than most Ollama models at the moment, and it’s very affordable.
