"""Module entrypoint supporting ``python -m perishables completions <shell>`` and CLI delegation."""

from __future__ import annotations

import subprocess
import sys
from typing import Sequence

from . import PerishablesError, completions, find_binary

_COMPLETIONS_HELP = """\
python -m perishables completions - Emit shell tab-completion scripts for perishables

USAGE:
  python -m perishables completions <bash|zsh|fish|powershell>

SHELLS:
  bash        GNU Bash completion script
  zsh         Zsh #compdef script
  fish        Fish completion rules
  powershell  PowerShell Register-ArgumentCompleter script
"""


def main(argv: Sequence[str] | None = None) -> int:
    """Run ``completions`` directly or forward arguments to the ``perishables`` binary."""
    args = list(sys.argv[1:] if argv is None else argv)
    if args and args[0] == "completions":
        sub = args[1:]
        if "-h" in sub or "--help" in sub:
            sys.stdout.write(_COMPLETIONS_HELP)
            return 0
        if len(sub) != 1:
            sys.stderr.write(
                "usage: python -m perishables completions <bash|zsh|fish|powershell>\n"
            )
            return 2
        try:
            sys.stdout.write(completions(sub[0]))
        except PerishablesError as exc:
            sys.stderr.write(f"{exc}\n")
            return exc.exit_code
        return 0

    try:
        exe = find_binary()
    except FileNotFoundError as exc:
        sys.stderr.write(f"{exc}\n")
        return 2
    completed = subprocess.run([str(exe), *args], check=False)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
