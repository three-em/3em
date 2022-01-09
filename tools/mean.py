#!/usr/bin/env python

import argparse
import json
import numpy as np
import matplotlib.pyplot as plt

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("file", help="JSON file with benchmark results")

args = parser.parse_args()

plt.rcdefaults()
fig, ax = plt.subplots()

entities = ('3em JS', '3em EVM', '3em WASM', 'Smartweave JS', '3em JS (fh)')
y_pos = np.arange(len(entities))

with open(args.file) as f:
    results = json.load(f)["results"]

performance = [b["mean"] for b in results]

ax.barh(y_pos, performance, align='center')
ax.set_yticks(y_pos, labels=entities)
ax.invert_yaxis()  # labels read top-to-bottom
ax.set_xlabel('mean time (s)')

plt.show()
