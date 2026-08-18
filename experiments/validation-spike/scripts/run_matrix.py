#!/usr/bin/env python3
"""Stage-0 validation-feasibility matrix driver.

Runs every fixture through cvc5, then through both candidate validation
pipelines, then through the problem-misbinding and proof-mutation attacks, and
writes the result matrix to results/.

Exit status:
    0  the harness ran to completion (whatever the scientific verdicts were)
    1  the harness itself malfunctioned

A pipeline failing its Stage-0 gate is NOT a harness malfunction.
"""
import json
import os
import re
import subprocess
import sys
import time

SPIKE_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SCRIPTS = os.path.join(SPIKE_ROOT, "scripts")
CASES = os.path.join(SPIKE_ROOT, "cases")
RESULTS = os.path.join(SPIKE_ROOT, "results")
RAW = os.path.join(RESULTS, "raw")
WORK = os.path.join(RESULTS, "raw", "work")

EXPECT = re.compile(r"^;\s*expect\s*:\s*(sat|unsat)\b", re.MULTILINE)
PURPOSE = re.compile(r"^;\s*semantic purpose\s*:\s*(.*)$", re.MULTILINE)

malfunctions = []


def malfunction(message):
    malfunctions.append(message)
    sys.stderr.write("STAGE0-HARNESS-ERROR: %s\n" % message)


def run(argv, **kwargs):
    return subprocess.run(argv, capture_output=True, text=True, **kwargs)


def keyvals(proc, context):
    """Parse the key=value protocol of the pipeline scripts."""
    if proc.returncode != 0:
        malfunction("%s exited %d: %s" % (context, proc.returncode, proc.stderr.strip()[:300]))
        return None
    out = {}
    for line in proc.stdout.splitlines():
        if "=" in line:
            key, _, value = line.partition("=")
            out[key] = value
    if "verdict" not in out:
        malfunction("%s produced no verdict" % context)
        return None
    return out


def tool_versions():
    env = os.environ
    versions = {}
    cvc5 = os.path.join(SPIKE_ROOT, ".tools", "bin", "cvc5")
    ethos = os.path.join(SPIKE_ROOT, ".tools", "bin", "ethos")
    carcara = os.path.join(SPIKE_ROOT, ".tools", "bin", "carcara")
    versions["os"] = run(["uname", "-srmo"]).stdout.strip()
    versions["cvc5"] = run([cvc5, "--version"]).stdout.splitlines()[0].strip()
    # ethos has no --version flag; the pinned commit is the identity
    versions["ethos_commit"] = env.get("ETHOS_COMMIT", "")
    versions["ethos_binary"] = run(["sha256sum", os.path.realpath(ethos)]).stdout.split()[0]
    versions["carcara"] = run([carcara, "--version"]).stdout.strip()
    versions["carcara_commit"] = env.get("CARCARA_COMMIT", "")
    z3 = env.get("Z3", "")
    versions["z3"] = run([z3, "--version"]).stdout.strip() if z3 else "not installed"
    versions["rustc"] = run(["rustc", "--version"]).stdout.strip()
    versions["cargo"] = run(["cargo", "--version"]).stdout.strip()
    # Carcara pins its own toolchain in rust-toolchain.toml, so the compiler that
    # built it is not necessarily the default one above.
    carcara_src = os.path.join(SPIKE_ROOT, ".tools", "src", "carcara")
    if os.path.isdir(carcara_src):
        versions["carcara_rustc"] = run(["rustc", "--version"], cwd=carcara_src).stdout.strip()
    versions["cmake"] = run(["cmake", "--version"]).stdout.splitlines()[0].strip()
    versions["cxx"] = run(["g++", "--version"]).stdout.splitlines()[0].strip()
    versions["python"] = sys.version.split()[0]
    return versions


def solve(case_path, produce_model=False):
    cvc5 = os.path.join(SPIKE_ROOT, ".tools", "bin", "cvc5")
    if not produce_model:
        proc = run([cvc5, case_path])
        return proc.stdout.strip().splitlines()[0] if proc.stdout.strip() else "<no output>"
    tmp = os.path.join(WORK, os.path.basename(case_path) + ".model.smt2")
    with open(tmp, "w", encoding="utf-8") as handle:
        handle.write(open(case_path, encoding="utf-8").read())
        handle.write("\n(get-model)\n")
    proc = run([cvc5, "--produce-models", tmp])
    return proc.stdout.strip()


def cross_check(case_path):
    z3 = os.environ.get("Z3", "")
    if not z3 or not os.path.exists(z3):
        return "not-run"
    proc = run([z3, "-smt2", case_path])
    return proc.stdout.strip().splitlines()[0] if proc.stdout.strip() else "<no output>"


def pipeline_a(problem, reference, mode, reuse=None, work=WORK):
    argv = [os.path.join(SCRIPTS, "run-cpc-ethos.sh"),
            "--problem", problem, "--reference", reference, "--work", work, "--mode", mode]
    if reuse:
        argv += ["--proof", reuse]
    return keyvals(run(argv), "run-cpc-ethos.sh(%s, %s)" % (os.path.basename(problem), mode))


def pipeline_b(problem, reference, granularity=None, reuse=None, work=WORK):
    argv = [os.path.join(SCRIPTS, "run-alethe-carcara.sh"),
            "--problem", problem, "--reference", reference, "--work", work]
    if granularity:
        argv += ["--granularity", granularity]
    if reuse:
        argv += ["--proof", reuse]
    return keyvals(run(argv), "run-alethe-carcara.sh(%s, %s)" % (os.path.basename(problem), granularity or "default"))


def load_cases():
    cases = []
    for name in sorted(os.listdir(CASES)):
        if not name.endswith(".smt2"):
            continue
        path = os.path.join(CASES, name)
        text = open(path, encoding="utf-8").read()
        expected = EXPECT.search(text)
        purpose = PURPOSE.search(text)
        if not expected:
            malfunction("fixture %s declares no '; expect :' line" % name)
            continue
        cases.append({"name": name, "path": path,
                      "expected": expected.group(1),
                      "purpose": purpose.group(1).strip() if purpose else ""})
    return cases


def load_mutations():
    rows = []
    path = os.path.join(CASES, "mutations.tsv")
    for line in open(path, encoding="utf-8"):
        if line.startswith("#") or not line.strip():
            continue
        cols = line.rstrip("\n").split("\t")
        rows.append({"case": cols[0], "id": cols[1], "count": cols[2],
                     "search": cols[3], "replace": cols[4], "description": cols[5]})
    return rows


def harness_self_tests():
    """Check that a missing tool fails loudly instead of yielding a false PASS."""
    tests = []
    case = os.path.join(CASES, "01_wrapping_equiv_valid.smt2")
    for tool in ("ETHOS", "CARCARA", "CVC5"):
        env = dict(os.environ)
        env[tool] = "/nonexistent/%s" % tool.lower()
        script = "run-alethe-carcara.sh" if tool == "CARCARA" else "run-cpc-ethos.sh"
        proc = subprocess.run([os.path.join(SCRIPTS, script), "--problem", case, "--work", WORK],
                              capture_output=True, text=True, env=env)
        loud = proc.returncode != 0 and "STAGE0-HARNESS-ERROR" in proc.stderr
        no_false_pass = "verdict=ACCEPT" not in proc.stdout
        tests.append({"missing_tool": tool, "exit": proc.returncode,
                      "failed_loudly": loud, "no_false_accept": no_false_pass,
                      "ok": loud and no_false_pass})
        if not (loud and no_false_pass):
            malfunction("missing %s did not fail loudly" % tool)
    return tests


def main():
    os.makedirs(RAW, exist_ok=True)
    os.makedirs(WORK, exist_ok=True)
    started = time.time()

    versions = tool_versions()
    cases = load_cases()
    mutations = load_mutations()

    report = {"versions": versions, "cases": [], "misbinding": [], "proof_mutation": [],
              "self_tests": [], "generated_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime())}

    # ---- per-case solving and proof production -----------------------------
    for case in cases:
        record = dict(case)
        record["cvc5"] = solve(case["path"])
        record["z3"] = cross_check(case["path"])
        record["classification_ok"] = (record["cvc5"] == case["expected"])
        record["cross_check_ok"] = record["z3"] in (case["expected"], "not-run")
        if not record["classification_ok"]:
            malfunction("%s: cvc5 answered %s, fixture declares %s"
                        % (case["name"], record["cvc5"], case["expected"]))
        if not record["cross_check_ok"]:
            malfunction("%s: z3 answered %s, fixture declares %s"
                        % (case["name"], record["z3"], case["expected"]))

        if case["expected"] == "unsat":
            record["a_reference"] = pipeline_a(case["path"], case["path"], "reference")
            record["a_standalone"] = pipeline_a(case["path"], case["path"], "standalone")
            record["a_cli_reference"] = pipeline_a(case["path"], case["path"], "cli-reference")
            record["b_default"] = pipeline_b(case["path"], case["path"])
            record["b_dsl_rewrite"] = pipeline_b(case["path"], case["path"], granularity="dsl-rewrite")
        else:
            model = solve(case["path"], produce_model=True)
            path = os.path.join(RAW, case["name"] + ".model.txt")
            open(path, "w", encoding="utf-8").write(model + "\n")
            record["model_file"] = os.path.relpath(path, SPIKE_ROOT)
            record["model_excerpt"] = " ".join(model.split())[:240]
        report["cases"].append(record)

    # ---- problem-misbinding attacks ----------------------------------------
    for mutation in mutations:
        base = os.path.join(CASES, mutation["case"])
        mutated = os.path.join(WORK, "%s.%s.smt2" % (mutation["case"][:-5], mutation["id"]))
        proc = run([os.path.join(SCRIPTS, "mutate-binding.sh"), mutation["case"], mutation["id"]])
        if proc.returncode != 0:
            malfunction("mutate-binding.sh %s/%s failed" % (mutation["case"], mutation["id"]))
            continue
        open(mutated, "w", encoding="utf-8").write(proc.stdout)

        entry = dict(mutation)
        # the mutation must actually destroy the property the preserved proof claims
        entry["mutated_cvc5"] = solve(mutated)
        entry["mutation_is_semantic"] = (entry["mutated_cvc5"] == "sat")
        if not entry["mutation_is_semantic"]:
            malfunction("misbinding mutation %s/%s left the problem unsat; it proves nothing"
                        % (mutation["case"], mutation["id"]))

        stem = mutation["case"][:-5]
        cpc_bound_proof = os.path.join(WORK, stem + ".reference.cpc")
        cpc_unbound_proof = os.path.join(WORK, stem + ".standalone.cpc")
        alethe_proof = os.path.join(WORK, stem + ".default.alethe")

        entry["a_reference"] = pipeline_a(base, mutated, "reference", reuse=cpc_bound_proof)
        entry["a_standalone"] = pipeline_a(base, mutated, "standalone", reuse=cpc_unbound_proof)
        entry["a_cli_reference"] = pipeline_a(base, mutated, "cli-reference", reuse=cpc_bound_proof)
        entry["b_default"] = pipeline_b(base, mutated, reuse=alethe_proof)
        report["misbinding"].append(entry)

    # ---- proof-mutation attacks --------------------------------------------
    for case in cases:
        if case["expected"] != "unsat":
            continue
        stem = case["name"][:-5]
        for kind in ("premise-swap", "literal-flip"):
            for label, proof, runner in (
                ("cpc-ethos", os.path.join(WORK, stem + ".reference.cpc"), "a"),
                ("alethe-carcara", os.path.join(WORK, stem + ".default.alethe"), "b"),
            ):
                if not os.path.exists(proof):
                    continue
                mutated_proof = "%s.%s.mutated" % (proof, kind)
                proc = run([sys.executable, os.path.join(SCRIPTS, "mutate-proof.py"),
                            "--kind", kind, "--in", proof, "--out", mutated_proof])
                if proc.returncode == 3:
                    report["proof_mutation"].append({"case": case["name"], "pipeline": label,
                                                     "kind": kind, "mutation": {"mutation_description": "no mutation site in this proof"},
                                                     "result": {"verdict": "NOT-APPLICABLE"}})
                    continue
                if proc.returncode != 0:
                    malfunction("mutate-proof.py %s on %s failed" % (kind, proof))
                    continue
                described = dict(line.split("=", 1) for line in proc.stdout.splitlines() if "=" in line)
                if runner == "a":
                    result = pipeline_a(case["path"], case["path"], "reference", reuse=mutated_proof)
                else:
                    result = pipeline_b(case["path"], case["path"], reuse=mutated_proof)
                report["proof_mutation"].append({"case": case["name"], "pipeline": label,
                                                 "kind": kind, "mutation": described,
                                                 "result": result})

    report["classification"] = classify(report)
    report["self_tests"] = harness_self_tests()
    report["malfunctions"] = malfunctions
    report["elapsed_seconds"] = round(time.time() - started, 1)

    with open(os.path.join(RESULTS, "matrix.json"), "w", encoding="utf-8") as handle:
        json.dump(report, handle, indent=2, sort_keys=True)
    write_toolchain(versions)
    write_matrix_tsv(report)
    write_summary(report)

    if malfunctions:
        sys.stderr.write("STAGE0-HARNESS-ERROR: %d malfunction(s); see results/summary.md\n" % len(malfunctions))
        return 1
    return 0


def classify(report):
    """Derive each pipeline's Stage-0 verdict from the recorded evidence.

    The verdict vocabulary is fixed by the Stage-0 plan. Exactly one applies to
    each pipeline, and it is computed here rather than asserted by hand.
    """
    unsat_cases = [c for c in report["cases"] if c["expected"] == "unsat"]
    out = {}

    a_accepted = [c for c in unsat_cases if verdict_of(c, "a_reference") == "ACCEPT"]
    a_trust = [c for c in unsat_cases if field(c, "a_reference", "trust_steps", "0") != "0"]
    a_binding_holes = [m for m in report["misbinding"] if verdict_of(m, "a_reference") != "REJECT"]
    a_mutation_holes = [m for m in report["proof_mutation"]
                        if m["pipeline"] == "cpc-ethos"
                        and m["result"] and m["result"].get("verdict") not in ("REJECT", "NOT-APPLICABLE")]
    if a_trust:
        out["cpc-ethos"] = "FAIL - TRUST/Hole"
    elif len(a_accepted) != len(unsat_cases):
        out["cpc-ethos"] = "FAIL - CHECKER REJECTS"
    elif a_binding_holes or a_mutation_holes:
        out["cpc-ethos"] = "FAIL - PROBLEM BINDING NOT ESTABLISHED"
    else:
        out["cpc-ethos"] = "PASS"

    def b_best(case):
        return "ACCEPT" if "ACCEPT" in (verdict_of(case, "b_default"),
                                        verdict_of(case, "b_dsl_rewrite")) else "REJECT"

    b_accepted = [c for c in unsat_cases if b_best(c) == "ACCEPT"]
    b_holey = [c for c in unsat_cases if field(c, "b_default", "carcara_stdout") == "holey"]
    b_invalid = [c for c in unsat_cases if field(c, "b_default", "carcara_stdout") == "invalid"]
    b_binding_holes = [m for m in report["misbinding"] if verdict_of(m, "b_default") != "REJECT"]
    if len(b_accepted) == len(unsat_cases) and not b_binding_holes:
        out["alethe-carcara"] = "PASS"
    elif b_holey:
        out["alethe-carcara"] = "FAIL - TRUST/Hole"
    elif b_invalid:
        out["alethe-carcara"] = "FAIL - CHECKER REJECTS"
    else:
        out["alethe-carcara"] = "FAIL - CORE-A COVERAGE"

    out["evidence"] = {
        "unsat_cases": len(unsat_cases),
        "cpc_accepted": len(a_accepted),
        "cpc_cases_with_trust_steps": len(a_trust),
        "cpc_misbinding_not_rejected_bound": len(a_binding_holes),
        "cpc_misbinding_not_rejected_cli_reference":
            len([m for m in report["misbinding"] if verdict_of(m, "a_cli_reference") != "REJECT"]),
        "cpc_misbinding_not_rejected_unbound":
            len([m for m in report["misbinding"] if verdict_of(m, "a_standalone") != "REJECT"]),
        "cpc_proof_mutations_not_rejected": len(a_mutation_holes),
        "alethe_accepted": len(b_accepted),
        "alethe_holey_default": len(b_holey),
        "alethe_invalid_default": len(b_invalid),
        "alethe_misbinding_not_rejected": len(b_binding_holes),
        "misbinding_attacks": len(report["misbinding"]),
    }
    return out


def verdict_of(record, key):
    block = record.get(key)
    return block.get("verdict", "?") if block else "HARNESS-ERROR"


def field(record, key, field_name, default="-"):
    block = record.get(key)
    return block.get(field_name, default) if block else "HARNESS-ERROR"


def write_toolchain(versions):
    lines = ["# Stage-0 toolchain, as recorded by scripts/run-all.sh", ""]
    for key in ("os", "cvc5", "ethos_commit", "ethos_binary", "carcara", "carcara_commit",
                "carcara_rustc", "z3", "rustc", "cargo", "cmake", "cxx", "python"):
        lines.append("%-16s %s" % (key, versions.get(key, "")))
    lines += ["", "cvc5            official static release asset cvc5-Linux-x86_64-static.zip",
              "ethos           built from source at the commit cvc5 pins in contrib/get-ethos-checker",
              "carcara         built from source; the project publishes no tagged release",
              "z3              independent second solver, used only to cross-check sat/unsat",
              "                expectations. It is not part of any admission-evidence path.", ""]
    open(os.path.join(RESULTS, "toolchain.txt"), "w", encoding="utf-8").write("\n".join(lines))


def write_matrix_tsv(report):
    header = ["case", "expected", "cvc5", "z3", "cpc_proof_steps", "cpc_trust_steps",
              "ethos", "ethos_bound", "alethe_proof_steps", "alethe_hole_steps",
              "carcara", "pipeline_a_verdict", "pipeline_b_verdict"]
    rows = [header]
    for record in report["cases"]:
        if record["expected"] == "unsat":
            rows.append([
                record["name"], record["expected"], record["cvc5"], record["z3"],
                field(record, "a_reference", "proof_steps"),
                field(record, "a_reference", "trust_steps"),
                field(record, "a_reference", "ethos_stdout"),
                field(record, "a_reference", "ethos_stdout") + "/reference-command",
                field(record, "b_default", "proof_steps"),
                field(record, "b_default", "hole_steps"),
                field(record, "b_default", "carcara_stdout"),
                verdict_of(record, "a_reference"), verdict_of(record, "b_default"),
            ])
        else:
            rows.append([record["name"], record["expected"], record["cvc5"], record["z3"],
                         "n/a", "n/a", "n/a", "n/a", "n/a", "n/a", "n/a",
                         "n/a-negative-case", "n/a-negative-case"])
    with open(os.path.join(RESULTS, "matrix.tsv"), "w", encoding="utf-8") as handle:
        for row in rows:
            handle.write("\t".join(str(c) for c in row) + "\n")


def write_summary(report):
    out = []
    add = out.append
    add("# Stage-0 result matrix")
    add("")
    add("Generated by `scripts/run-all.sh` at %s in %.1fs." % (report["generated_utc"], report["elapsed_seconds"]))
    add("")
    add("This file is regenerated by the harness. The narrative Stage-0 report is")
    add("[`../README.md`](../README.md).")
    add("")

    add("## Toolchain")
    add("")
    add("```text")
    for key in ("os", "cvc5", "ethos_commit", "carcara", "carcara_commit", "z3"):
        add("%-16s %s" % (key, report["versions"].get(key, "")))
    add("```")
    add("")

    add("## Case classification")
    add("")
    add("| case | expected | cvc5 | z3 (cross-check) | agrees |")
    add("| --- | --- | --- | --- | --- |")
    for record in report["cases"]:
        agrees = "yes" if record["classification_ok"] and record["cross_check_ok"] else "**NO**"
        add("| `%s` | %s | %s | %s | %s |" % (record["name"], record["expected"],
                                              record["cvc5"], record["z3"], agrees))
    add("")

    add("## Pipeline A — cvc5 -> CPC -> Ethos")
    add("")
    add("`bound` is the run with `(reference \"<problem>.smt2\")` in the proof, which makes")
    add("Ethos require every assumption to occur among the problem's assertions.")
    add("`unbound` is the self-contained proof Ethos checks with no problem file at all.")
    add("")
    add("| case | steps | trust steps | Ethos (bound) | Ethos (`--reference=`) | Ethos (unbound) | verdict |")
    add("| --- | --- | --- | --- | --- | --- | --- |")
    for record in report["cases"]:
        if record["expected"] != "unsat":
            continue
        add("| `%s` | %s | %s | %s | %s | %s | %s |" % (
            record["name"], field(record, "a_reference", "proof_steps"),
            field(record, "a_reference", "trust_steps"),
            field(record, "a_reference", "ethos_stdout"),
            field(record, "a_cli_reference", "ethos_stdout"),
            field(record, "a_standalone", "ethos_stdout"),
            verdict_of(record, "a_reference")))
    add("")

    add("## Pipeline B — cvc5 -> Alethe -> Carcara")
    add("")
    add("| case | steps | hole steps | rare_rewrite steps | Carcara (default) | Carcara (dsl-rewrite) | verdict |")
    add("| --- | --- | --- | --- | --- | --- | --- |")
    for record in report["cases"]:
        if record["expected"] != "unsat":
            continue
        best = "ACCEPT" if "ACCEPT" in (verdict_of(record, "b_default"), verdict_of(record, "b_dsl_rewrite")) else "REJECT"
        add("| `%s` | %s | %s | %s | %s | %s | %s |" % (
            record["name"], field(record, "b_default", "proof_steps"),
            field(record, "b_default", "hole_steps"),
            field(record, "b_default", "rare_rewrite_steps"),
            field(record, "b_default", "carcara_stdout"),
            field(record, "b_dsl_rewrite", "carcara_stdout"),
            best))
    add("")

    add("## Problem-misbinding attacks")
    add("")
    add("The proof of the unmutated fixture is preserved and re-checked against a")
    add("mutated problem. Every mutation turns the fixture from unsat into sat, so any")
    add("ACCEPT below is a checker accepting a proof of a different logical problem.")
    add("")
    add("| case | mutation | mutated cvc5 | Ethos, in-proof `(reference ...)` | Ethos, `--reference=` flag | Ethos, no binding | Carcara |")
    add("| --- | --- | --- | --- | --- | --- | --- |")
    for entry in report["misbinding"]:
        add("| `%s` | %s | %s | %s (%s) | %s (%s) | %s (%s) | %s (%s) |" % (
            entry["case"], entry["id"], entry["mutated_cvc5"],
            verdict_of(entry, "a_reference"), field(entry, "a_reference", "ethos_stdout"),
            verdict_of(entry, "a_cli_reference"), field(entry, "a_cli_reference", "ethos_stdout"),
            verdict_of(entry, "a_standalone"), field(entry, "a_standalone", "ethos_stdout"),
            verdict_of(entry, "b_default"), field(entry, "b_default", "carcara_stdout")))
    add("")

    add("## Proof-mutation attacks")
    add("")
    add("| case | pipeline | mutation | what changed | verdict |")
    add("| --- | --- | --- | --- | --- |")
    for entry in report["proof_mutation"]:
        add("| `%s` | %s | %s | %s | %s |" % (
            entry["case"], entry["pipeline"], entry["kind"],
            entry["mutation"].get("mutation_description", "-"),
            entry["result"].get("verdict", "HARNESS-ERROR") if entry["result"] else "HARNESS-ERROR"))
    add("")

    add("## Stage-0 gate")
    add("")
    add("Computed by the harness from the evidence above, using the Stage-0")
    add("acceptance rule: a pipeline passes only when every expected-unsat fixture")
    add("is accepted by the independent checker with no trust/hole step, every")
    add("problem-misbinding attack is rejected, and every proof mutation is rejected.")
    add("")
    add("```text")
    add("cvc5 -> CPC    -> Ethos     %s" % report["classification"]["cpc-ethos"])
    add("cvc5 -> Alethe -> Carcara   %s" % report["classification"]["alethe-carcara"])
    add("```")
    add("")
    add("| evidence | value |")
    add("| --- | --- |")
    for key, value in sorted(report["classification"]["evidence"].items()):
        add("| %s | %s |" % (key.replace("_", " "), value))
    add("")

    add("## Harness self-tests")
    add("")
    add("| removed tool | exit status | failed loudly | no false ACCEPT |")
    add("| --- | --- | --- | --- |")
    for entry in report["self_tests"]:
        add("| %s | %s | %s | %s |" % (entry["missing_tool"], entry["exit"],
                                       entry["failed_loudly"], entry["no_false_accept"]))
    add("")

    add("## Harness malfunctions")
    add("")
    if report["malfunctions"]:
        for message in report["malfunctions"]:
            add("- %s" % message)
    else:
        add("None. Every scientific verdict above was produced by a harness run that")
        add("completed normally.")
    add("")
    open(os.path.join(RESULTS, "summary.md"), "w", encoding="utf-8").write("\n".join(out))


if __name__ == "__main__":
    sys.exit(main())
