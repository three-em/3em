#!/usr/bin/env python

import argparse
import json
import numpy as np
import matplotlib.pyplot as plt

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("file", help="JSON file with benchmark results")

args = parser.parse_args()

with open(args.file) as f:
    results = json.load(f)["results"]

performance = [b["times"] for b in results]

fig, ax = plt.subplots()

three_em_times = performance[0]
ypoints_0 = np.array(three_em_times)
ax.plot(ypoints_0, color = 'b', label='3em JS')
ax.legend()

three_evm_times = performance[1]
ypoints_1 = np.array(three_evm_times)
ax.plot(ypoints_1, color = 'g', label='3em EVM')
ax.legend()

three_wasm_times = performance[2]
ypoints_2 = np.array(three_wasm_times)
ax.plot(ypoints_2, color = 'r', label='3em WASM')
ax.legend()

smartweave_times = performance[3]
ypoints_3 = np.array(smartweave_times)
ax.plot(ypoints_3, color = 'r', label='Smartweave JS')
ax.legend()

ax.set_xlabel('iterations')
ax.set_ylabel('time taken per iteration (s)')

plt.show()