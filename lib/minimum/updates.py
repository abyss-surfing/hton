"""Seven-day update policy and fast-forward of the deployed source checkout.

The protected stamp records the last successful install/update. The update timer
runs daily and performs work only when that stamp is at least one week old, so an
installation that is already refreshed by an explicit ``scripts/install`` is not
updated again inside the same week. Code fast-forwarding is limited to the
checkout's ``main`` branch and ``origin/main``; local commits and dirty paths are
never rewritten.
"""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import time

from storage import InstallError, Store

INTERVAL_SECONDS = 7 * 24 * 60 * 60
STAMP = "update.json"
BRANCH = "main"
REMOTE = "origin"
GIT_TIMEOUT = 300


def _now(now=None):
    return int(time.time() if now is None else now)


def record(store: Store, kind: str, now=None) -> None:
    """Record one successful install/update completion."""
    store.save(STAMP, {
        "schemaVersion": 1,
        "lastSuccessAt": _now(now),
        "kind": kind,
        "intervalSeconds": INTERVAL_SECONDS,
    })


def last_success(store: Store):
    """Return the last successful timestamp, or None when absent/invalid."""
    try:
        value = store.json(STAMP)
    except InstallError:
        return None
    if (not isinstance(value, dict) or value.get("schemaVersion") != 1
            or type(value.get("lastSuccessAt")) is not int
            or value["lastSuccessAt"] < 0):
        return None
    return value["lastSuccessAt"]


def due(store: Store, now=None, interval: int = INTERVAL_SECONDS) -> bool:
    """A missing stamp or a clock that moved backwards counts as due."""
    current = _now(now)
    last = last_success(store)
    if last is None or current < last:
        return True
    return current - last >= interval


def remaining(store: Store, now=None, interval: int = INTERVAL_SECONDS) -> int:
    last = last_success(store)
    if last is None:
        return 0
    return max(0, interval - (_now(now) - last))


def _git(root: Path, *arguments: str):
    git = shutil.which("git")
    if git is None:
        return None
    environment = dict(os.environ)
    environment.update({"GIT_TERMINAL_PROMPT": "0", "LC_ALL": "C"})
    return subprocess.run(
        [git, "-C", str(root), "-c", "safe.directory=" + str(root), *arguments],
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=GIT_TIMEOUT,
        env=environment, check=False,
    )


def _head(root: Path):
    result = _git(root, "rev-parse", "--verify", "HEAD")
    if result is None or result.returncode != 0:
        return None
    return result.stdout.decode("ascii", "replace").strip() or None


def fast_forward(root: Path) -> dict:
    """Fast-forward ``main`` to ``origin/main`` without rewriting local work.

    Returns a redacted status record. Git's own output is never propagated; the
    caller only needs the bounded outcome and reason.
    """
    root = Path(root)
    if not (root / ".git").exists():
        return {"status": "skipped", "reason": "not-a-git-checkout"}
    branch = _git(root, "symbolic-ref", "--quiet", "--short", "HEAD")
    if branch is None:
        return {"status": "failed", "reason": "git-unavailable"}
    if branch.returncode != 0:
        return {"status": "skipped", "reason": "detached-head"}
    if branch.stdout.decode("utf-8", "replace").strip() != BRANCH:
        return {"status": "skipped", "reason": "not-main-branch"}
    before = _head(root)
    fetched = _git(root, "fetch", "--quiet", "--no-tags", REMOTE, BRANCH)
    if fetched is None:
        return {"status": "skipped", "reason": "git-unavailable"}
    if fetched.returncode != 0:
        return {"status": "failed", "reason": "fetch-failed", "before": before}
    merged = _git(root, "merge", "--ff-only", "--quiet", REMOTE + "/" + BRANCH)
    if merged is None:
        return {"status": "skipped", "reason": "git-unavailable"}
    if merged.returncode != 0:
        return {"status": "failed", "reason": "not-fast-forward", "before": before}
    after = _head(root)
    return {
        "status": "updated" if before != after else "current",
        "reason": "fast-forward",
        "before": before,
        "after": after,
    }


def _report(result: dict, quiet: bool) -> None:
    if quiet:
        return
    status = result.get("status")
    if status == "updated":
        print("Source updated to origin/main.", file=sys.stderr)
    elif status == "skipped":
        print("Source update skipped: " + str(result.get("reason", "unavailable")) + "; current source retained.", file=sys.stderr)
    elif status == "failed":
        print("Source update failed: " + str(result.get("reason", "unknown")) + "; current source retained.", file=sys.stderr)


def _open_work(work_root: Path):
    if not work_root.exists():
        return None
    return Store(work_root)


def main(argv=None) -> int:
    parser = argparse.ArgumentParser(prog="updates.py", description="HTON update schedule and source fast-forward")
    commands = parser.add_subparsers(dest="command", required=True)
    due_parser = commands.add_parser("due", help="exit 0 when the weekly update is due, 1 when it is not")
    due_parser.add_argument("--work-root", required=True)
    due_parser.add_argument("--now", type=int)
    due_parser.add_argument("--interval", type=int, default=INTERVAL_SECONDS)
    due_parser.add_argument("--json", action="store_true")
    forward_parser = commands.add_parser("fast-forward", help="fast-forward main from origin/main")
    forward_parser.add_argument("--root", required=True)
    forward_parser.add_argument("--required", action="store_true", help="fail when the source update cannot be performed")
    forward_parser.add_argument("--quiet", action="store_true")
    forward_parser.add_argument("--json", action="store_true")
    args = parser.parse_args(argv)

    if args.command == "due":
        work_root = Path(args.work_root)
        try:
            store = _open_work(work_root)
            if store is None:
                result = {"due": True, "reason": "no-stamp", "remainingSeconds": 0}
            else:
                try:
                    ready = due(store, args.now, args.interval)
                    result = {"due": ready, "remainingSeconds": remaining(store, args.now, args.interval)}
                finally:
                    store.close()
        except (InstallError, OSError):
            print("ERROR: update schedule cannot be read safely", file=sys.stderr)
            return 2
        if args.json:
            print(json.dumps(result, sort_keys=True))
        elif not result["due"]:
            print("recent", file=sys.stderr)
        return 0 if result["due"] else 1

    try:
        result = fast_forward(Path(args.root))
    except (OSError, subprocess.SubprocessError):
        result = {"status": "failed", "reason": "git-error"}
    _report(result, args.quiet)
    if args.json:
        print(json.dumps(result, sort_keys=True))
    if args.required and result["status"] == "failed":
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
