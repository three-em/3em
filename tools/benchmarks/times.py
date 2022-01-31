#!/usr/bin/env python

import argparse
import json
import os
import numpy as np
import matplotlib.pyplot as plt

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("file", help="JSON file with benchmark results")

args = parser.parse_args()

with open(args.file, encoding="utf8") as f:
    results = json.load(f)["results"]

labels = {
    "../../target/release/bench": {
        "name": "3em JS",
        "color": "black",
    },
    "../../target/release/bench_evm": {
        "name": "3em EVM",
        "color": "green",
    },
    "../../target/release/bench_wasm": {
        "name": "3em WASM",
        "color": "red",
    },
    "../../target/release/bench_fh": {
        "name": "3em JS (fh)",
        "color": "blue",
    },
    "node ./smartweave/index.js": {
        "name": "Smartweave JS",
        "color": "orange",
    },
}

fig, ax = plt.subplots(figsize=(8, 4))

for result in results:
    ax.plot(np.array([round(t * 1000) for t in result["times"]]),
            color=labels[result["command"]]["color"],
            label=labels[result["command"]]["name"])

ax.legend()
plt.xticks(range(0, 20, 1), labels=range(1, 21, 1))
ax.set_xlim(left=-1, right=20)
ax.set_xlabel('runs')
ax.set_ylabel('time taken per run (ms)')

plt.savefig(os.path.realpath(os.path.dirname(__file__)) + '/bench_times.png')
