#!/usr/bin/env python

import argparse
import json
import os
import numpy as np
import matplotlib.pyplot as plt

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("file", help="JSON file with benchmark results")

args = parser.parse_args()

plt.rcdefaults()
fig, ax = plt.subplots(figsize=(10, 5))

labels = {
    "../../target/release/bench": "3em JS",
    "../../target/release/bench_evm": "3em EVM",
    "../../target/release/bench_wasm": "3em WASM",
    "../../target/release/bench_fh": "3em JS (fh)",
    "node ./smartweave/index.js": "Smartweave JS",
}

y_pos = np.arange(len(labels))

with open(args.file, encoding="utf8") as f:
    results = sorted(json.load(f)["results"], key=lambda x: x["mean"])


performance = [round(b["mean"] * 1000) for b in results]

bars = ax.barh(y_pos, performance, align='center')
ax.set_yticks(y_pos, labels=map(lambda x: labels[x["command"]], results))
ax.invert_yaxis()  # labels read top-to-bottom
ax.set_xlabel('mean time (ms)')
ax.set_title('Time taken to calculate contract state\n(lower is better)')

ax.bar_label(bars, fmt='%d ms', padding=8)
ax.set_xlim(right=8000)  # adjust xlim to fit labels

plt.savefig(os.path.realpath(os.path.dirname(__file__)) + '/bench_mean.png')
