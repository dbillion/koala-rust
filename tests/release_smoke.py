#!/usr/bin/env python3
"""Smoke-test a release bundle and the separately downloadable sqlite-vec library."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def query_vec_version(binary: Path, expected_version: str | None, extension: Path | None) -> str:
    with tempfile.TemporaryDirectory(prefix="koala-release-smoke-") as state:
        env = os.environ.copy()
        env.pop("KOALA_VEC0_PATH", None)
        if extension is not None:
            env["KOALA_VEC0_PATH"] = str(extension)
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
        mode = f" with {extension.name}" if extension else " from the bundle"
        raise RuntimeError(f"sqlite-vec startup/query failed{mode}: {result.stderr[-4000:]}")
    try:
        rows = json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(f"SQL command did not return JSON: {result.stdout[-2000:]!r}") from error

    if not rows or not rows[0].get("version"):
        raise RuntimeError(f"vec_version() returned no version: {result.stdout!r}")
    actual_version = rows[0]["version"]
    if expected_version and actual_version != expected_version:
        raise RuntimeError(
            f"wrong sqlite-vec version: expected {expected_version}, got {actual_version}"
        )
    return actual_version


def main() -> int:
    if len(sys.argv) not in (2, 3):
        print(
            "usage: release_smoke.py PATH_TO_KOALA_BINARY [PATH_TO_RAW_VEC0_LIBRARY]",
            file=sys.stderr,
        )
        return 2

    binary = Path(sys.argv[1]).resolve()
    raw_extension = Path(sys.argv[2]).resolve() if len(sys.argv) == 3 else None
    if not binary.is_file():
        raise FileNotFoundError(f"release binary not found: {binary}")

    extension_name = "vec0.dll" if os.name == "nt" else (
        "vec0.dylib" if sys.platform == "darwin" else "vec0.so"
    )
    bundled_extension = binary.parent / extension_name
    if not bundled_extension.is_file():
        raise FileNotFoundError(f"bundled sqlite-vec library not found: {bundled_extension}")
    if raw_extension is not None and not raw_extension.is_file():
        raise FileNotFoundError(f"raw sqlite-vec library not found: {raw_extension}")

    help_result = subprocess.run(
        [str(binary), "--help"],
        capture_output=True,
        text=True,
        timeout=30,
        cwd=binary.parent,
    )
    if help_result.returncode:
        raise RuntimeError(f"--help failed: {help_result.stderr[-2000:]}")

    expected_version = os.environ.get("SQLITE_VEC_VERSION")
    bundled_version = query_vec_version(binary, expected_version, None)
    print(f"PASS bundled release: --help + vec_version() -> {bundled_version}")

    if raw_extension is not None:
        raw_version = query_vec_version(binary, expected_version, raw_extension)
        if raw_version != bundled_version:
            raise RuntimeError(
                f"raw extension version {raw_version} differs from bundle {bundled_version}"
            )
        print(f"PASS raw release library: {raw_extension.name} -> {raw_version}")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
