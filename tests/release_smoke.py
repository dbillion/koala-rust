#!/usr/bin/env python3
"""Smoke-test a native release bundle and its adjacent sqlite-vec library."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: release_smoke.py PATH_TO_KOALA_BINARY", file=sys.stderr)
        return 2

    binary = Path(sys.argv[1]).resolve()
    if not binary.is_file():
        raise FileNotFoundError(f"release binary not found: {binary}")
    extension_name = "vec0.dll" if os.name == "nt" else (
        "vec0.dylib" if sys.platform == "darwin" else "vec0.so"
    )
    extension_path = binary.parent / extension_name
    if not extension_path.is_file():
        raise FileNotFoundError(f"bundled sqlite-vec library not found: {extension_path}")

    help_result = subprocess.run(
        [str(binary), "--help"],
        capture_output=True,
        text=True,
        timeout=30,
        cwd=binary.parent,
    )
    if help_result.returncode:
        raise RuntimeError(f"--help failed: {help_result.stderr[-2000:]}")

    with tempfile.TemporaryDirectory(prefix="koala-release-smoke-") as state:
        env = os.environ.copy()
        env.pop("KOALA_VEC0_PATH", None)
        # Keep relative extension lookup inside the archive, not the source tree.
        result = subprocess.run(
            [
                str(binary), "--state-dir", state, "sql",
                "SELECT vec_version() AS version", "-f", "json",
            ],
            capture_output=True,
            text=True,
            timeout=60,
            env=env,
            cwd=binary.parent,
        )

    if result.returncode:
        raise RuntimeError(f"sqlite-vec startup/query failed: {result.stderr[-4000:]}")
    try:
        rows = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"SQL command did not return JSON: {result.stdout[-2000:]!r}") from error

    if not rows or not rows[0].get("version"):
        raise RuntimeError(f"vec_version() returned no version: {result.stdout!r}")
    expected_version = os.environ.get("SQLITE_VEC_VERSION")
    actual_version = rows[0]["version"]
    if expected_version and actual_version != expected_version:
        raise RuntimeError(
            f"wrong sqlite-vec version: expected {expected_version}, got {actual_version}"
        )

    print(f"PASS release bundle: --help + vec_version() -> {actual_version}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
