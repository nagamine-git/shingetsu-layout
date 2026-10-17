#!/usr/bin/env python3
"""Generate Google IME / Mozc romanization table from shingetsu_analyzer.json

The table is tab-separated: input <TAB> output [<TAB> next input].
Google 日本語入力 reads the import file as the only rule set and, once a rule's input is also the
beginning of a longer rule, waits for the next key. It never falls back to the shorter rule when the
next key leads nowhere (a prefix that has no rule of its own). Step 3 below adds the rules that make
such prefixes fall back to the shorter rule.
"""

import json

# Rules the analyzer cannot express:
#   - ー is also typed with the "-" key (the TSV has the same row; the analyzer only has ★p)
#   - も is also typed with kk (☆☆) and ら with dd (★★)
EXTRA_RULES = [("-", "ー"), ("kk", "も"), ("dd", "ら")]


def build_rules(data):
    """Return [(input, output, next_input)] sorted by (len(input), input)."""
    rules = {}

    def add(inp, out, pending=""):
        if inp in rules:
            raise ValueError(f"duplicate input {inp!r}: {rules[inp][0]!r} and {out!r}")
        rules[inp] = (out, pending)

    # 1. every analyzer entry that is typed with keys only
    for char, entry in data["conversion"].items():
        keys = entry.get("keys", [])
        shift = entry.get("shift", [])

        # Skip space-based entries (not representable in romanization table)
        if "space" in keys:
            continue

        add("".join(shift) + "".join(keys), char)

    # 小書き ゃ/ゅ/ょ は専用キーのほかに「親文字 + ゛」でも打てるようにする（ぁぃぅぇぉ・ゎ と同じ規則）
    dakuten = "".join(data["conversion"]["゛"]["keys"])
    seq_of = {char: inp for inp, (char, _) in rules.items()}
    for small, parent in {"ゃ": "や", "ゅ": "ゆ", "ょ": "よ"}.items():
        alias = seq_of[parent] + dakuten
        if alias not in rules:
            add(alias, small)

    # 2. extra rules
    for inp, out in EXTRA_RULES:
        add(inp, out)

    # 3. dead-end prefixes: for every proper prefix p of a rule that has no rule of its own, fall back to
    #    the longest shorter rule q that is a prefix of p: p -> output(q), next input = the rest of p.
    #    When the rest of p is itself a rule (rli = で + い, the beginning of でぃ = rlil), p -> output(q) +
    #    output(rest) instead: a rule that still has longer rules behind it only keeps its output while it
    #    waits for the next key, not its next input.
    fallback = {}
    for inp in rules:
        for i in range(1, len(inp)):
            p = inp[:i]
            if p in rules or p in fallback:
                continue
            q = next((inp[:j] for j in range(i - 1, 0, -1) if inp[:j] in rules), None)
            if q is not None:
                out, pending = rules[q]
                rest = p[len(q):]
                if not pending and rest in rules:
                    fallback[p] = (out + rules[rest][0], rules[rest][1])
                else:
                    fallback[p] = (out, pending + rest)
    rules.update(fallback)

    return sorted(((inp, out, pending) for inp, (out, pending) in rules.items()),
                  key=lambda r: (len(r[0]), r[0]))


def main():
    with open("shingetsu_analyzer.json") as f:
        data = json.load(f)

    rules = build_rules(data)

    with open("shingetsu-romantable.txt", "w") as f:
        for inp, out, pending in rules:
            f.write(f"{inp}\t{out}\t{pending}\n" if pending else f"{inp}\t{out}\n")

    print(f"Generated {len(rules)} entries to shingetsu-romantable.txt")


if __name__ == "__main__":
    main()
