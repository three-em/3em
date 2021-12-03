import { benchmark3em } from "./benchmark_provider.ts";
import { ContractTypes, Providers } from "./model.ts";

const benchmarksRuns = {
  wasm: await benchmark3em({
    type: ContractTypes.WASM,
    runs: 10,
    file: "./3em-wasm-benchmark.json",
    provider: Providers.EM3,
  }),
  js: await benchmark3em({
    type: ContractTypes.JS,
    runs: 10,
    file: "./3em-js-benchmark.json",
    provider: Providers.EM3,
  }),
  smartweaveJs: await benchmark3em({
    type: ContractTypes.JS,
    runs: 10,
    file: "./smartweave-js-benchmark.json",
    provider: Providers.SMARTWEAVE,
  }),
  redstoneJs: await benchmark3em({
    type: ContractTypes.JS,
    runs: 10,
    file: "./redstone-js-benchmark.json",
    provider: Providers.REDSTONE,
  }),
};

console.log(benchmarksRuns);
