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

performance = [b["times"] for b in results]

fig, ax = plt.subplots()

for result in results:
    ypoints_0 = np.array([round(t * 1000) for t in result["times"]])
    ax.plot(ypoints_0,
            color=labels[result["command"]]["color"],
            label=labels[result["command"]]["name"])
    ax.legend()

ax.set_xlim(right=20)  # adjust xlim to fit labels
ax.set_xlabel('iterations')
ax.set_ylabel('time taken per iteration (s)')

plt.savefig(os.path.realpath(os.path.dirname(__file__)) + '/bench_runs.png')
