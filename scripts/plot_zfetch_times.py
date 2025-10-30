#!/usr/bin/env python3

"""
Run the ZFetch release binary multiple times and build a graph of execution times.

Usage:
    python scripts/plot_zfetch_times.py --runs 20 --output zfetch_times.png

The script expects the release binary at `target/release/ZFetch`. It parses the
`Execution time: ...` line printed by the binary on stdout and stores the total
duration (in milliseconds) for each run. A simple matplotlib chart is written to
the requested output path.
"""

from __future__ import annotations

import argparse
import re
import statistics
import subprocess
from pathlib import Path
from typing import List

import matplotlib.pyplot as plt

EXECUTION_RE = re.compile(r"Execution time:\s*([0-9.]+)\s*([a-zμ]+)", re.IGNORECASE)


def normalize_to_ms(value: float, unit: str) -> float:
    unit = unit.lower()
    if unit.startswith("ms"):
        return value
    if unit.startswith("µs") or unit.startswith("us"):
        return value / 1000.0
    if unit.startswith("ns"):
        return value / 1_000_000.0
    if unit.startswith("s"):
        return value * 1000.0
    raise ValueError(f"Unknown time unit: {unit}")


def run_once(binary: Path, extra_args: List[str]) -> float:
    result = subprocess.run(
        [str(binary), *extra_args],
        capture_output=True,
        text=True,
        check=False,
    )

    if result.returncode != 0:
        raise RuntimeError(
            f"ZFetch exited with code {result.returncode}.\n"
            f"stdout:\n{result.stdout}\n\nstderr:\n{result.stderr}"
        )

    match = EXECUTION_RE.search(result.stdout)
    if not match:
        raise RuntimeError(
            "Failed to parse execution time from ZFetch output. "
            "Ensure the binary prints the execution line."
        )
    value, unit = match.groups()
    return normalize_to_ms(float(value), unit)


def compute_moving_average(values: List[float], window: int) -> List[float]:
    if window <= 1 or window > len(values):
        return values[:]

    averaged: List[float] = []
    cumulative = 0.0
    for i, value in enumerate(values):
        cumulative += value
        if i >= window:
            cumulative -= values[i - window]
            averaged.append(cumulative / window)
        elif i == window - 1:
            averaged.append(cumulative / window)
        else:
            averaged.append(cumulative / (i + 1))
    return averaged


def plot_times(times_ms: List[float], output_path: Path, smooth_window: int) -> None:
    runs = list(range(1, len(times_ms) + 1))

    plt.figure(figsize=(10, 4))
    plt.plot(
        runs,
        times_ms,
        marker="o",
        markersize=4,
        linewidth=0.8,
        alpha=0.4,
        color="#1f77b4",
        label="Samples",
    )

    if smooth_window > 1:
        smoothed = compute_moving_average(times_ms, smooth_window)
        plt.plot(
            runs,
            smoothed,
            linewidth=2.0,
            color="#2ca02c",
            label=f"Moving Avg ({smooth_window})",
        )

    plt.title("ZFetch Execution Time (ms)")
    plt.xlabel("Run")
    plt.ylabel("Duration (ms)")
    plt.grid(True, linestyle="--", linewidth=0.5, alpha=0.6)

    mean = statistics.mean(times_ms)
    plt.axhline(
        mean, color="#ff7f0e", linestyle="--", linewidth=1, label=f"Mean {mean:.2f} ms"
    )
    plt.legend(loc="upper right")

    plt.tight_layout()
    output_path.parent.mkdir(parents=True, exist_ok=True)
    plt.savefig(output_path)
    plt.close()


def main() -> None:
    parser = argparse.ArgumentParser(description="Benchmark ZFetch release binary.")
    parser.add_argument(
        "--runs",
        type=int,
        default=10,
        help="Number of times to execute the binary (default: 10).",
    )
    parser.add_argument(
        "--binary",
        type=Path,
        default=Path("target/release/ZFetch"),
        help="Path to the ZFetch binary (default: target/release/ZFetch).",
    )
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("zfetch_execution_times.png"),
        help="Where to write the resulting plot image.",
    )
    parser.add_argument(
        "--args",
        nargs=argparse.REMAINDER,
        help="Additional arguments to forward to the ZFetch binary.",
    )
    parser.add_argument(
        "--smooth-window",
        type=int,
        default=25,
        help="Window size for moving-average smoothing (default: 25; use 1 to disable).",
    )

    args = parser.parse_args()

    binary = args.binary
    if not binary.exists():
        raise FileNotFoundError(
            f"Could not find ZFetch binary at {binary}. "
            "Build it with `cargo build --release` first."
        )

    extra_args = args.args if args.args else []
    times_ms: List[float] = []
    for i in range(args.runs):
        duration = run_once(binary, extra_args)
        print(f"Run {i + 1}/{args.runs}: {duration:.2f} ms")
        times_ms.append(duration)

    plot_times(times_ms, args.output, args.smooth_window)
    print(f"Wrote plot to {args.output}")
    print(f"Average: {statistics.mean(times_ms):.2f} ms")
    if len(times_ms) > 1:
        print(f"Stddev: {statistics.pstdev(times_ms):.2f} ms")


if __name__ == "__main__":
    main()
