FORBIDDEN = [
    "rm ", "dd ", "mkfs", ":", "|", ">", "<", "&&", "||",
    "curl", "wget", "bash", "sh"
]

ALLOWED_PREFIXES = (
    "pacman ",
    "paru ",
    "systemctl ",
    "ip ",
    "launch ",
    "true",
)

def validate(cmd: str) -> None:
    for bad in FORBIDDEN:
        if bad in cmd:
            raise ValueError(f"Blocked unsafe command: {cmd}")

    if not cmd.startswith(ALLOWED_PREFIXES):
        raise ValueError(f"Command not allowed: {cmd}")
