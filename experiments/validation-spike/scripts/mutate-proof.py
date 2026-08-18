#!/usr/bin/env python3
"""Controlled textual mutation of a proof, for the Stage-0 proof-mutation attack.

usage: mutate-proof.py --kind KIND --in PROOF --out MUTATED

KIND is one of:

  premise-swap   Replace one premise reference of the last step that cites
                 premises with a different, earlier, already-proved step id.
                 The mutated file still parses and every step id still exists,
                 so a rejection can only come from checking the rule
                 application, not from the parser.

  literal-flip   Flip the last bit of the last bit-vector literal appearing in
                 a step line. This changes a justified conclusion or rule
                 argument while leaving the proof structure intact.

Both mutations work on CPC and on Alethe proofs: the two formats share the
`(step <id> ... :premises (...) ...)` shape and SMT-LIB bit-vector literals.

Prints key=value lines describing what was mutated. Exit status 0 on success,
3 when this proof has no site for the requested mutation kind (a recorded
outcome, not a harness malfunction), 2 on a usage error.
"""
import argparse
import re
import sys

STEP_ID = re.compile(r"^\((?:step|step-pop)\s+(\S+)")
PREMISES = re.compile(r":premises\s*\(([^)]*)\)")
BV_LITERAL = re.compile(r"#b[01]+")


def collect_ids(lines):
    ids = []
    for line in lines:
        m = STEP_ID.match(line)
        if m:
            ids.append(m.group(1))
    return ids


def premise_swap(lines):
    ids = collect_ids(lines)
    for index in range(len(lines) - 1, -1, -1):
        m = PREMISES.search(lines[index])
        if not m:
            continue
        cited = m.group(1).split()
        if not cited:
            continue
        # every id proved strictly before this step, minus the ones it already cites
        this_id = STEP_ID.match(lines[index])
        if this_id is None:
            continue
        earlier = ids[: ids.index(this_id.group(1))]
        candidates = [i for i in earlier if i not in cited]
        if not candidates:
            continue
        replacement = candidates[0]
        old = cited[0]
        new_premises = " ".join([replacement] + cited[1:])
        mutated = lines[index][: m.start(1)] + new_premises + lines[index][m.end(1):]
        return index, lines[index], mutated, "premise %s replaced by %s" % (old, replacement)
    return None


def literal_flip(lines):
    # Prefer a literal inside a step: there the mutation changes a justified
    # conclusion or a rule argument and nothing else. Both formats also share
    # terms through aliases, so when no step carries a literal we fall back to
    # the alias definition. That second site is reported explicitly, because a
    # literal shared with an assumed formula makes the mutated proof a proof of
    # a different problem as well as a badly justified one.
    for site in ("(step", "(define"):
        for index in range(len(lines) - 1, -1, -1):
            if not lines[index].startswith(site):
                continue
            matches = list(BV_LITERAL.finditer(lines[index]))
            if not matches:
                continue
            m = matches[-1]
            literal = m.group(0)
            flipped = literal[:-1] + ("0" if literal[-1] == "1" else "1")
            mutated = lines[index][: m.start()] + flipped + lines[index][m.end():]
            return (index, lines[index], mutated,
                    "bit-vector literal %s replaced by %s in a %s line"
                    % (literal, flipped, site[1:]))
    return None


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--kind", required=True, choices=["premise-swap", "literal-flip"])
    parser.add_argument("--in", dest="src", required=True)
    parser.add_argument("--out", dest="dst", required=True)
    args = parser.parse_args()

    lines = open(args.src, encoding="utf-8").read().splitlines()
    result = (premise_swap if args.kind == "premise-swap" else literal_flip)(lines)
    if result is None:
        # Exit 3 means "this proof has no site for this mutation kind". It is a
        # recorded outcome, not a harness malfunction.
        sys.stderr.write("no %s mutation site in %s\n" % (args.kind, args.src))
        return 3

    index, original, mutated, description = result
    lines[index] = mutated
    open(args.dst, "w", encoding="utf-8").write("\n".join(lines) + "\n")
    print("mutation_kind=%s" % args.kind)
    print("mutation_line=%d" % (index + 1))
    print("mutation_description=%s" % description)
    print("mutation_before=%s" % original.strip()[:200])
    print("mutation_after=%s" % mutated.strip()[:200])
    return 0


if __name__ == "__main__":
    sys.exit(main())
