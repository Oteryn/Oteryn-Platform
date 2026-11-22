#!/usr/bin/env bash
# Run the browser qualification against a coverage-instrumented portal.
set +x
set -euo pipefail
umask 077

e2e="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
portal="$(dirname "$e2e")"
cd "$portal"
: "${PORTAL_E2E_ADMIN_DATABASE_URL:?Set the disposable local PostgreSQL administrator URL}"
target_dir="${PORTAL_E2E_LLVM_TARGET_DIR:-$portal/target/e2e-llvm-cov}"
mkdir -p "$target_dir"
target_dir="$(cd "$target_dir" && pwd)"
run_dir="$(mktemp -d "${TMPDIR:-/tmp}/portal-e2e-llvm.XXXXXXXX")"
printf 'Coverage run directory: %s\n' "$run_dir"
mkdir "$run_dir/profiles" "$run_dir/build-profiles" "$run_dir/auth"

if ! (cd "$e2e" && PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1 npm ci --ignore-scripts --no-audit --no-fund \
    --cache "$target_dir/npm-cache") >"$run_dir/npm.log" 2>&1; then
    echo 'Playwright dependency installation failed. See npm.log in the run directory.' >&2
    exit 1
fi
unset CARGO_ENCODED_RUSTFLAGS
if ! LLVM_PROFILE_FILE="$run_dir/build-profiles/%m.profraw" RUSTFLAGS="-C instrument-coverage" \
    CARGO_PROFILE_DEV_DEBUG=0 CARGO_TARGET_DIR="$target_dir" \
    cargo build --locked -p oteryn-portal >"$run_dir/build.log" 2>&1; then
    echo 'Instrumented build failed. See build.log in the run directory.' >&2
    exit 1
fi

export PORTAL_E2E_LLVM_BINARY="$target_dir/debug/oteryn-portal"
export PORTAL_E2E_LLVM_PROFILES="$run_dir/profiles"
export PORTAL_E2E_AUTH_DIRECTORY="$run_dir/auth"
export PORTAL_E2E_SCRATCH="$run_dir/harness"
export PLAYWRIGHT_JSON_OUTPUT_FILE="$run_dir/playwright.json"
cat >"$run_dir/portal" <<'WRAPPER'
#!/usr/bin/env bash
# Give startup, HTTP and worker profiles distinct names so they can be verified.
export LLVM_PROFILE_FILE="$PORTAL_E2E_LLVM_PROFILES/$1-%p-%m.profraw"
exec "$PORTAL_E2E_LLVM_BINARY" "$@"
WRAPPER
chmod +x "$run_dir/portal"

launcher=''
stop_launcher() {
    if [[ -n "$launcher" ]]; then
        local pid="$launcher"
        launcher=''
        kill -TERM "$pid" 2>/dev/null || true
        wait "$pid"
    fi
}
trap stop_launcher EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
PORTAL_E2E_BINARY="$run_dir/portal" node "$e2e/local.mjs" >"$run_dir/harness.log" 2>&1 &
launcher=$!

# The browser launcher exposes an HTTPS proxy in front of the portal's HTTP listener.
if ! curl --fail --silent --insecure --retry 30 --retry-connrefused --retry-all-errors \
    --retry-max-time 90 --max-time 5 https://localhost:18443/healthz -o /dev/null ||
    ! kill -0 "$launcher" 2>/dev/null; then
    echo 'Browser harness failed readiness. See harness.log and harness/serve.log in the run directory.' >&2
    exit 1
fi

suite_status=0
(cd "$e2e" && node run.mjs "$run_dir/harness/env.json" test --output "$run_dir/test-results") \
    >"$run_dir/playwright.log" 2>&1 || suite_status=$?
stop_launcher

shopt -s nullglob
serve_profiles=("$run_dir"/profiles/serve-*.profraw)
worker_profiles=("$run_dir"/profiles/worker-*.profraw)
profiles=("${serve_profiles[@]}" "${worker_profiles[@]}")
if (( ${#serve_profiles[@]} == 0 || ${#worker_profiles[@]} == 0 )); then
    echo 'Expected nonempty serve and worker profile sets after graceful shutdown.' >&2
    exit 1
fi
for profile in "${profiles[@]}"; do
    [[ -s "$profile" ]] || { echo 'An empty runtime profile was produced.' >&2; exit 1; }
done
if cargo llvm-cov --version >/dev/null 2>&1; then
    # Isolate the executable and runtime profiles from Cargo build artifacts.
    report_target="$run_dir/report-target"
    mkdir -p "$report_target/debug"
    cp --reflink=auto "$PORTAL_E2E_LLVM_BINARY" "$report_target/debug/"
    ln -s "${profiles[@]}" "$report_target/"
    CARGO_LLVM_COV_TARGET_DIR="$report_target" cargo llvm-cov report --json \
        --output-path "$run_dir/llvmcov.json"
else
    tool_dir="$(rustc --print target-libdir)/../bin"
    if [[ ! -x "$tool_dir/llvm-profdata" || ! -x "$tool_dir/llvm-cov" ]]; then
        rustup component add llvm-tools
    fi
    "${LLVM_PROFDATA:-$tool_dir/llvm-profdata}" merge --sparse "${profiles[@]}" -o "$run_dir/portal.profdata"
    mapfile -d '' sources < <(find "$portal/crates" "$portal/bin" -type f -path '*/src/*.rs' -print0)
    "${LLVM_COV:-$tool_dir/llvm-cov}" export "$PORTAL_E2E_LLVM_BINARY" --instr-profile "$run_dir/portal.profdata" \
        "${sources[@]}" >"$run_dir/llvmcov.json"
fi

python3 - "$portal" "$run_dir" "$suite_status" "${#serve_profiles[@]}" "${#worker_profiles[@]}" <<'PYTHON'
from bisect import bisect_right
from collections import Counter, defaultdict
from functools import cache
import json
from pathlib import Path
import re
import subprocess
import sys

portal, run = map(Path, sys.argv[1:3])
suite_status, serve_profiles, worker_profiles = map(int, sys.argv[3:])
coverage = json.loads((run / "llvmcov.json").read_text())["data"][0]
playwright = json.loads((run / "playwright.json").read_text())
metrics = ("lines", "functions", "regions")
totals = defaultdict(lambda: {metric: Counter(covered=0, count=0) for metric in metrics})


@cache
def source_path(filename):
    path = Path(filename)
    if path.is_absolute():
        if not path.is_relative_to(portal):
            return None
        path = path.relative_to(portal)
    if len(path.parts) >= 4 and path.parts[0] in ("bin", "crates") and path.parts[2] == "src":
        return path
    return None


for file in coverage["files"]:
    path = source_path(file["filename"])
    if path is None:
        continue
    for metric in metrics:
        for key in ("covered", "count"):
            totals[path.parts[1]][metric][key] += file["summary"][metric][key]

# Match coverable Rust declarations to their first code region. Async functions
# also have a poll body. Count execution of that body when one is mapped.
declaration = re.compile(
    r"(?m)^[ \t]*(?P<public>pub(?:\([^\n)]+\))?\s+)?"
    r"(?P<qualifiers>(?:(?:async|const|unsafe|extern(?:\s+\"[^\"]*\")?)\s+)*)"
    r"fn\s+(?P<name>\w+)\s*(?=[<(])"
)
registrations = re.compile(
    r"\b(?:get|post|put|patch|delete|head|options|any)\(\s*"
    r"(?P<handler>(?:[a-zA-Z_]\w*::)*[a-zA-Z_]\w*)(?:::[<][^;()\n]*[>])?\s*\)"
)
sources = {
    path.relative_to(portal): path.read_text()
    for directory in ("crates", "bin")
    for path in (portal / directory).glob("*/src/**/*.rs")
    if "tests" not in path.relative_to(portal).parts and not path.name.endswith("tests.rs")
}
route_start = re.compile(r"\.route(?:_service)?\s*\(")
route_tokens = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:[^"\\]|\\.)*"|[()]', re.S)


def route_calls(source):
    for route in route_start.finditer(source):
        depth = 1
        for token in route_tokens.finditer(source, route.end()):
            if token[0] == "(":
                depth += 1
            elif token[0] == ")":
                depth -= 1
                if depth == 0:
                    yield source[route.end():token.start()]
                    break

declarations = defaultdict(dict)
handlers = defaultdict(set)
route_crates = set()
for path, source in sources.items():
    crate = path.parts[1]
    for match in declaration.finditer(source):
        declarations[path][source.count("\n", 0, match.start()) + 1] = match
    # Detect crates registering Axum routes, including private API and Gateway
    # route crates that the browser route inventory intentionally omits.
    for args in route_calls(source):
        route_crates.add(crate)
        for match in registrations.finditer(args):
            handlers[crate].add(match["handler"].split("::")[-1])
# The coin router constructs a MethodRouter before passing it to .route().
handlers["wallet"].add("handle")

route_crates = sorted(route_crates)
missing = [crate for crate in route_crates if not totals[crate]["functions"]["count"]]

entries = defaultdict(list)
polls = defaultdict(list)
file_regions = defaultdict(list)
askama = Counter(mapped=0, covered=0)
for function in coverage["functions"]:
    if not function["regions"]:
        continue
    start = function["regions"][0]
    path = source_path(function["filenames"][start[5]])
    if path is None:
        continue
    name = function["name"]
    if "6askama8Template" in name and "write_into" in name:
        askama["mapped"] += 1
        askama["covered"] += function["count"] > 0
    if path not in sources:
        continue
    line = start[0]
    for region in function["regions"]:
        if region[7] == 0 and source_path(function["filenames"][region[5]]) == path:
            file_regions[path].append(region)
    if line in declarations[path]:
        decl = declarations[path][line]
        # v0 symbols include the length-prefixed Rust item name. A closure
        # generated by lowering an async fn has a nested closure path (NC).
        item = decl["name"]
        if f"{len(item)}{item}" in name:
            records = polls if "_RNC" in name and "async" in decl["qualifiers"] else entries
            records[path, line].append(function["count"])

uncovered = defaultdict(list)
for path, line in sorted(entries.keys() | polls.keys()):
    if any(polls.get((path, line), entries.get((path, line), []))):
        continue
    decl = declarations[path][line]
    crate = path.parts[1]
    category = "Axum handler" if decl["name"] in handlers[crate] else "pub" if decl["public"] else "private"
    uncovered[crate].append((str(path), line, decl["name"], category))

# These dispatch sites choose a mutation or permission from an action supplied
# by the form or route. Use each arm's source span to find LLVM code regions.
dispatch = {
    "admin/src/bazaar.rs": ["match self.action.as_str()"],
    "admin/src/commerce/http.rs": ["match (action.as_str(), form.days, form.amount)"],
    "admin/src/players/http.rs": ["match form.action.as_str()"],
    "admin/src/web.rs": ["match form.action.as_str()"],
    "bazaar/src/items/web.rs": ["match action.as_str()"],
    "characters/src/services/browser.rs": ["match action.as_str()"],
    "community/src/referrals/web.rs": ["match form.kind.as_str()"],
    "minigames/src/collection_pvp/http.rs": ["match c.op.as_str()"],
    "playerdata/src/places/http.rs": ["match form.participating.as_str()"],
    "payments/src/coin_http.rs": ["match decision.action.as_str()"],
    "store/src/web_payments.rs": ["match action.as_str()"],
    "wiki/src/web.rs": ["match action.as_str()"],
    "world/src/admin.rs": ["match form.action.as_str()"],
}
tokens = re.compile(r'//[^\n]*|/\*.*?\*/|"(?:[^"\\]|\\.)*"|=>|[(){}\[\],]|[^\s(){}\[\],]+?', re.S)


def match_arms(source, opening):
    pattern = None
    arrow = None
    stack = []
    for token in tokens.finditer(source, opening + 1):
        value = token[0]
        if value.startswith(("//", "/*")):
            continue
        if not stack:
            if value in (",", "}"):
                if arrow is not None:
                    yield pattern, arrow, token.start()
                pattern = arrow = None
                if value == "}":
                    break
                continue
            if value == "=>":
                arrow = token.end()
                continue
            if pattern is None:
                pattern = token.start()
        if value in ("(", "{", "["):
            stack.append(value)
        elif value in (")", "}", "]"):
            top_level_block = (
                len(stack) == 1 and stack[-1] == "{" and arrow is not None
                and source[arrow:token.start()].lstrip().startswith("{")
            )
            stack.pop()
            if top_level_block:
                yield pattern, arrow, token.end()
                pattern = arrow = None


uncovered_arms = defaultdict(list)
unverified_arms = defaultdict(list)
for name, selectors in dispatch.items():
    path = Path("crates") / name
    source = sources[path]
    starts = [0, *(match.end() for match in re.finditer("\n", source))]

    def offset(line, column):
        # LLVM columns count UTF-8 bytes.
        start = starts[line - 1]
        end = starts[line] if line < len(starts) else len(source)
        return start + len(source[start:end].encode()[:column - 1].decode(errors="ignore"))

    regions = [(offset(r[0], r[1]), r[4]) for r in file_regions[path]]
    for selector in selectors:
        opening = source.index("{", source.index(selector) + len(selector))
        for pattern, arrow, end in match_arms(source, opening):
            label = " ".join(source[pattern:arrow - 2].split())
            arm_counts = [count for start, count in regions if arrow <= start < end]
            row = (str(path), bisect_right(starts, pattern), label)
            if not arm_counts:
                unverified_arms[path.parts[1]].append(row)
            elif not any(arm_counts):
                uncovered_arms[path.parts[1]].append(row)

def test_results(suite):
    for spec in suite.get("specs", []):
        yield from spec["tests"]
    for child in suite.get("suites", []):
        yield from test_results(child)


project_results = defaultdict(Counter)
for test in test_results({"suites": playwright["suites"]}):
    status = test["results"][-1]["status"] if test["results"] else "not_run"
    project_results[test["projectName"]][status] += 1
projects = sorted(project_results)
executed_projects = [
    project for project in projects
    if any(count for status, count in project_results[project].items()
           if status not in ("not_run", "skipped", "interrupted"))
]
stats = {name: playwright["stats"][name] for name in ("expected", "unexpected", "flaky", "skipped")}
revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=portal, text=True).strip()
parents = subprocess.check_output(["git", "log", "-1", "--format=%P"], cwd=portal, text=True).split()
tree = subprocess.check_output(["git", "rev-parse", "HEAD^{tree}"], cwd=portal, text=True).strip()
summary = {
    "source_commit": revision,
    "source_tree": tree,
    "source_parents": parents,
    "rustc": subprocess.check_output(["rustc", "--version"], cwd=portal, text=True).strip(),
    "playwright": {
        "projects": {project: dict(project_results[project]) for project in projects},
        "executed_projects": executed_projects,
        "exit_code": suite_status,
        "runner_errors": len(playwright.get("errors", [])),
        **stats,
    },
    "profraw": {"serve": serve_profiles, "worker": worker_profiles, "merged": serve_profiles + worker_profiles},
    "http_route_crates": route_crates,
    "http_route_crates_missing_functions": missing,
    "askama_generated_functions": dict(askama),
    "crates": {
        crate: {
            metric: {
                **values,
                "percent": round(100 * values["covered"] / values["count"], 2) if values["count"] else None,
            }
            for metric, values in coverage.items()
        }
        for crate, coverage in sorted(totals.items())
    },
}
(run / "llvmcov-summary.json").write_text(json.dumps(summary, indent=2) + "\n")

markdown = [
    "# Portal Rust coverage from Playwright",
    "",
    f"Measured source tree `{tree}` at `{revision}`.",
    f"Playwright exit code {suite_status}. Projects with completed cases {', '.join(executed_projects)}. "
    f"Results {stats['expected']} expected, {stats['unexpected']} unexpected, {stats['flaky']} flaky, "
    f"{stats['skipped']} skipped.",
    f"Merged {serve_profiles} nonempty HTTP server and {worker_profiles} nonempty worker profiles "
    "after launcher shutdown. Migration runs have their own profile and are excluded.",
    "Server startup, the harness health check and background worker activity contribute to the totals.",
    f"LLVM mapped functions in {len(route_crates) - len(missing)} of {len(route_crates)} crates "
    "that register HTTP routes. See the JSON summary for the full crate inventory.",
    "",
    "Run `portal-rs/e2e/ledger/llvm-cov.sh` using the local browser prerequisites and a disposable "
    "PostgreSQL administrator URL from [README.md](../README.md). The script prints a private run "
    "directory containing the regenerated ledger, tool logs and raw coverage. It sends SIGTERM to "
    "the browser launcher, which forwards its existing graceful SIGINT to the server and worker "
    "and waits for both.",
    "",
    "Individual Askama HTML template lines have no coverage mapping. Hand-written Rust "
    "rendering callers contribute to their crate totals. The workspace source map contains "
    f"{askama['mapped']} recognizable generated Askama Template write_into functions, "
    f"with {askama['covered']} exercised.",
    "",
    "| Crate | Lines | Functions | Regions |",
    "| --- | ---: | ---: | ---: |",
]
for crate, coverage in summary["crates"].items():
    cells = [
        f"{coverage[m]['covered']:,}/{coverage[m]['count']:,} ({coverage[m]['percent']:.2f}%)"
        if coverage[m]["count"] else "—"
        for m in metrics
    ]
    markdown.append(f"| {crate} | {' | '.join(cells)} |")

markdown += [
    "",
    "## Uncovered functions",
    "",
    "Zero-count source functions in `crates/*/src` and the portal binary are grouped below. Axum handler labels come "
    "from named method registrations. The list includes all mapped private Rust declarations so it "
    "also includes unexecuted callees. A declaration is uncovered when every mapped instantiation "
    "has zero calls. Async declarations use the poll body when mapped. LLVM totals also include "
    "compiler-generated functions and closures. Items absent from the linked coverage map have no count.",
]
for crate, coverage in sorted(uncovered.items()):
    markdown += ["", f"### {crate}", ""]
    for path, line, name, category in sorted(coverage, key=lambda row: ({"Axum handler": 0, "pub": 1, "private": 2}[row[3]], row)):
        markdown.append(f"- `{path}:{line}` · `{name}` ({category})")

markdown += [
    "",
    "## Uncovered form-action match arms",
    "",
    "An arm is uncovered when none of its mapped body regions executed. An arm with no mapped body "
    "region is reported as unmeasured. A Rust or-pattern shares its body counter across alternatives.",
]
for crate in sorted(uncovered_arms.keys() | unverified_arms.keys()):
    markdown += ["", f"### {crate}", ""]
    for path, line, label in uncovered_arms[crate]:
        markdown.append(f"- `{path}:{line}` · `{label}`")
    for path, line, label in unverified_arms[crate]:
        markdown.append(f"- `{path}:{line}` · `{label}` (unmeasured)")
(run / "LLVMCOV.md").write_text("\n".join(markdown) + "\n")

if missing:
    sys.exit("No LLVM functions mapped for HTTP route crates: " + ", ".join(missing))
if projects != ["en-1440", "en-390", "pl-1440", "pl-390"]:
    sys.exit("Playwright did not report all four required projects")
if any(counts["not_run"] or counts["interrupted"] for counts in project_results.values()):
    sys.exit("Playwright did not complete every collected test")
if stats["skipped"]:
    sys.exit("Playwright reported skipped tests")
PYTHON
exit "$suite_status"
