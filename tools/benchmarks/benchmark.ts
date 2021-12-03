import { benchmark3em } from "./benchmark_wasm.ts";

const benchmarksRuns = {
  wasm: await benchmark3em([], 10, "wasm"),
  js: await benchmark3em([], 10, "js"),
};

console.log(benchmarksRuns);
