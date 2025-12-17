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

archsim $ paru -S brave-bin <br>
:: Resolving AUR dependencies...<br>
:: Installing brave-bin<br>
archsim $<br>

2. Make the ai Command-line rewriting. I got it working, But only 70% of the time "it does not messup the commands it just put in extra stuff in it" 




#Plans
When smaller Ollama models become sufficiently capable, I want this to run fully offline. For now, however, using ChatGPT makes it far more powerful.
