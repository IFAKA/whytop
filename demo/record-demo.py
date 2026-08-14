#!/usr/bin/env python3
"""Record the deterministic whytop walkthrough with asciinema."""

from __future__ import annotations

import os
import pty
import shutil
import signal
import subprocess
import termios
import time


ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CAST = os.path.join(ROOT, "demo", "whytop.cast")


def require(name: str) -> None:
    if shutil.which(name) is None:
        raise SystemExit(f"error: required recording tool '{name}' is missing")


def set_size(fd: int, columns: int = 120, rows: int = 36) -> None:
    # TIOCSWINSZ is stable on macOS and Linux and keeps the cast dimensions fixed.
    setter = getattr(termios, "tcsetwinsize", None) or termios.tcsetwinsz
    setter(fd, (rows, columns))


def main() -> int:
    require("asciinema")
    os.makedirs(os.path.dirname(CAST), exist_ok=True)
    command = "WHYTOP_DEMO=1 cargo run --release"
    env = os.environ.copy()
    env.pop("WHYTOP_DEMO", None)
    argv = [
            "asciinema",
            "rec",
            "--stdin",
            "--overwrite",
            "--cols",
            "120",
            "--rows",
            "36",
            "-c",
            command,
            CAST,
        ]
    child, master = pty.fork()
    if child == 0:
        os.chdir(ROOT)
        os.execvpe(argv[0], argv, env)
    set_size(master)

    # (delay from previous action, bytes to send). The pauses are intentional:
    # they establish the table, make sorting legible, and show streaming.
    actions = [
        (2.5, b"j"),
        (1.5, b"\r"),
        (4.5, b""),
        (1.5, b"Is it safe?", 0.08),
        (0.5, b"\r"),
        (4.5, b""),
        (1.0, b"\x1b"),
        (1.0, b"q"),
    ]
    deadline = time.monotonic() + 70
    try:
        for action in actions:
            delay, payload, *interval = action
            time.sleep(delay)
            if time.monotonic() > deadline:
                raise RuntimeError("demo timed out while sending scripted input")
            if interval:
                for byte in payload:
                    os.write(master, bytes([byte]))
                    time.sleep(interval[0])
            elif payload:
                os.write(master, payload)
        remaining = deadline - time.monotonic()
        end = time.monotonic() + max(1, remaining)
        while time.monotonic() < end:
            waited, status = os.waitpid(child, os.WNOHANG)
            if waited:
                break
            time.sleep(0.1)
        else:
            raise subprocess.TimeoutExpired(argv, max(1, remaining))
    except (OSError, subprocess.TimeoutExpired, RuntimeError) as exc:
        os.killpg(child, signal.SIGTERM)
        try:
            os.waitpid(child, 0)
        except subprocess.TimeoutExpired:
            os.killpg(child, signal.SIGKILL)
            os.waitpid(child, 0)
        raise SystemExit(f"error: recording failed: {exc}") from exc
    finally:
        os.close(master)

    if not os.WIFEXITED(status) or os.WEXITSTATUS(status) != 0:
        raise SystemExit(f"error: asciinema exited with status {status}")
    print(f"wrote {CAST}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
