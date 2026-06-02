#!/usr/bin/env python3
"""
Extract OrchardZSA test blocks from a GitHub Actions job log.

Requires the GitHub CLI (gh) to be installed and authenticated.
See: https://cli.github.com

Blocks are only present in runs where zcash_tx_tool was invoked with -v,
which enables the debug logging that prints the full submitted block hex.

Commands:
    # Show recent runs and their IDs:
    ./scrape_zsa_blocks.py --list

    # Extract blocks from a specific run:
    ./scrape_zsa_blocks.py --run-id 26226362117

    # Extract from the latest successful run:
    ./scrape_zsa_blocks.py --run-id latest

    # Scan the last N successful runs and use the first one that has blocks:
    ./scrape_zsa_blocks.py --run-id latest 5
"""

import argparse
import json
import os
import re
import subprocess
import sys

OWNER         = "QED-it"
REPO          = "zcash_tx_tool"
WORKFLOW_FILE = "zebra-test-ci.yaml"
JOB_NAME      = "build-and-test"
BLOCK_PATTERN = re.compile(r'Full Request submitblock Full Body:\s*\["(.*?)"\]', re.DOTALL)
ANSI_ESCAPE   = re.compile(r'\x1B\[[0-?]*[ -/]*[@-~]')


def gh(*args, text=True):
    """Run a gh command and return stdout. Exits with error message on failure."""
    result = subprocess.run(["gh", *args], capture_output=True, text=text)
    if result.returncode != 0:
        sys.exit(f"gh error: {result.stderr.strip()}")
    return result.stdout


def gh_api(path, **params):
    """Call the GitHub API via gh and return parsed JSON."""
    query = "&".join(f"{k}={v}" for k, v in params.items())
    url = f"https://api.github.com/{path}?{query}" if params else f"https://api.github.com/{path}"
    return json.loads(gh("api", url))


def get_successful_runs(n):
    data = gh_api(f"repos/{OWNER}/{REPO}/actions/workflows/{WORKFLOW_FILE}/runs",
                  status="success", per_page=n)
    runs = data.get("workflow_runs", [])
    if not runs:
        sys.exit("No successful workflow runs found.")
    return runs


def fetch_log(run_id):
    jobs = gh_api(f"repos/{OWNER}/{REPO}/actions/runs/{run_id}/jobs").get("jobs", [])
    job = next((j for j in jobs if j["name"] == JOB_NAME), None)
    if not job:
        sys.exit(f"Job '{JOB_NAME}' not found in run {run_id}.")
    result = subprocess.run(
        ["gh", "api", f"repos/{OWNER}/{REPO}/actions/jobs/{job['id']}/logs"],
        capture_output=True, text=True
    )
    if result.returncode != 0:
        if "410" in result.stderr:
            print("  Log expired (HTTP 410), skipping.")
            return None
        sys.exit(f"gh error: {result.stderr.strip()}")
    return ANSI_ESCAPE.sub("", result.stdout)


def extract_blocks(log_text, out_dir):
    matches = BLOCK_PATTERN.findall(log_text)
    if not matches:
        return []
    os.makedirs(out_dir, exist_ok=True)
    for i, block in enumerate(matches, start=1):
        path = os.path.join(out_dir, f"orchard-zsa-workflow-block-{i}.txt")
        with open(path, "w") as f:
            f.write(block.replace("\n", "").strip())
        print(f"Block {i}: {len(block)//2} bytes -> {path}")
    print(f"\n{len(matches)} block(s) saved to {out_dir}/")
    return matches


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--run-id", nargs="+", metavar=("ID|latest", "N"),
                        help="run ID, 'latest', or 'latest N' to scan last N runs for blocks")
    parser.add_argument("--out-dir", default="zsa-blocks",
                        help="output directory for block files (default: zsa-blocks)")
    parser.add_argument("--list", type=int, metavar="N", nargs="?", const=10,
                        help="list the last N successful runs and exit (default 10)")
    args = parser.parse_args()

    if not any([args.list, args.run_id]):
        parser.print_help()
        sys.exit(0)

    if args.list is not None:
        print(f"{'RUN ID':<15}  {'CREATED':<20}  {'BRANCH':<40}  {'COMMIT'}")
        print("-" * 95)
        for run in get_successful_runs(args.list):
            print(f"{run['id']:<15}  {run['created_at']}  {run['head_branch']:<40}  {run['head_sha'][:12]}")
        return

    tokens = args.run_id
    if tokens[0] == "latest":
        n = int(tokens[1]) if len(tokens) > 1 else 1
        for run in get_successful_runs(n):
            print(f"Trying run id={run['id']} branch={run['head_branch']} created={run['created_at']}")
            log = fetch_log(run["id"])
            if log and extract_blocks(log, args.out_dir):
                return
            print("  No blocks found, trying next run...")
        sys.exit(f"No blocks found in the last {n} successful run(s).")
    else:
        run_id = int(tokens[0])
        log = fetch_log(run_id)
        if not log or not extract_blocks(log, args.out_dir):
            sys.exit(
                "No submitblock entries found in log.\n"
                "Blocks are only logged when zcash_tx_tool runs with -v.\n"
                "Use --list to find a suitable run, or try --run-id latest N."
            )


if __name__ == "__main__":
    main()
