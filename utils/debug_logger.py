#!/usr/bin/env python3
import sys
import subprocess
import threading
import time

LOGFILE = "protocol_log.txt"


def init_log():
    """Clear log file at start."""
    with open(LOGFILE, "w", encoding="utf-8") as f:
        f.write(
            f"=== New session started {time.strftime('%Y-%m-%d %H:%M:%S')} ===\n")


def log(direction: str, line: str):
    ts = time.strftime("%Y-%m-%d %H:%M:%S")
    with open(LOGFILE, "a", buffering=1, encoding="utf-8") as f:
        f.write(f"[{ts}] {direction}: {line}\n")


def forward_lines(src, dst, direction):
    """Forward line-based text between streams and log."""
    for line in src:
        dst.write(line)
        dst.flush()
        # Strip newline for logging readability
        log(direction, line.rstrip("\r\n"))


def main():
    real_prog = "/Users/samo/Documents/personal_projects/chess_engine/target/debug/sabertooth"

    init_log()

    proc = subprocess.Popen(
        real_prog,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        bufsize=1,  # line-buffered
        text=True   # decode as text
    )

    # Parent stdin → Child stdin
    t_in = threading.Thread(target=forward_lines, args=(
        sys.stdin, proc.stdin, "stdin -> child"))
    # Child stdout → Parent stdout
    t_out = threading.Thread(target=forward_lines, args=(
        proc.stdout, sys.stdout, "child -> stdout"))

    t_in.start()
    t_out.start()

    proc.wait()
    t_in.join()
    t_out.join()


if __name__ == "__main__":
    main()
