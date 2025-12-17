import os
from dotenv import load_dotenv
from openai import OpenAI

load_dotenv(dotenv_path=".env", override=True)

api_key = os.getenv("OPENAI_API_KEY")
client = OpenAI(api_key=api_key) if api_key and api_key.startswith("sk-") else None

SYSTEM_PROMPT = """You are an Arch Linux expert.

Installed packages (names only):
{installed}

Rules:
- Output ONLY ONE shell command
- No markdown
- No explanation
- Prefer pacman, then paru
- Do NOT reinstall installed packages
- If nothing should be done, output: true
- NEVER output dangerous commands (rm, dd, mkfs, pipes, redirects)
"""

def translate(user_input: str, state) -> str:
    if client is None:
        raise RuntimeError("OPENAI_API_KEY missing or invalid; set it in .env to enable AI translation")

    installed = "\n".join(sorted(state.installed_packages))
    system_prompt = SYSTEM_PROMPT.format(installed=installed)

    completion = client.chat.completions.create(
        model="gpt-5-mini",
        temperature=1,
        max_completion_tokens=50,
        messages=[
            {"role": "system", "content": system_prompt},
            {"role": "user", "content": user_input},
        ],
    )

    cmd = completion.choices[0].message.content.strip()

    # Treat empty output as "no-op"
    if not cmd:
        return "true"

    return cmd
