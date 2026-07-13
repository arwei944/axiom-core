#!/usr/bin/env python3
import subprocess
import sys

result = subprocess.run(
    ["cargo", "check", "--workspace"],
    cwd=r"D:\work\trae\axiom-core-project",
    capture_output=True,
    text=True,
    encoding="utf-8",
    errors="replace"
)

with open(r"D:\work\trae\axiom-core-project\cargo_check3.txt", "w", encoding="utf-8") as f:
    f.write(result.stdout)
    f.write(result.stderr)

print(f"Exit code: {result.returncode}")
sys.exit(result.returncode)
