#!/usr/bin/env python3
"""Generate Google IME / Mozc romanization table from shingetsu_analyzer.json"""

import json

def main():
    with open("shingetsu_analyzer.json") as f:
        data = json.load(f)

    lines = []
    for char, entry in data["conversion"].items():
        keys = entry.get("keys", [])
        shift = entry.get("shift", [])

        # Skip space-based entries (not representable in romanization table)
        if "space" in keys:
            continue

        input_seq = "".join(shift) + "".join(keys)
        lines.append((input_seq, char))

    lines.sort(key=lambda x: (len(x[0]), x[0]))

    with open("shingetsu-romantable.txt", "w") as f:
        for inp, out in lines:
            f.write(f"{inp}\t{out}\n")

    print(f"Generated {len(lines)} entries to shingetsu-romantable.txt")

if __name__ == "__main__":
    main()
