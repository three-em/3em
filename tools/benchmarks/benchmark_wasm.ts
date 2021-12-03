import { benchmark, JS_CMD, maxMinAvg, WASM_CMD } from "./util.ts";
import { StatsResult } from "./model.ts";

const getRuns = (defaultRuns?: number): number => {
  const benchmarkRuns = Deno.env.get("WASM_RUNS");
  return defaultRuns || (benchmarkRuns ? parseInt(benchmarkRuns) : undefined) ||
    10;
};

export const benchmark3em = async (
  extraParameters: Array<string> = [],
  nruns: number | undefined,
  type: "wasm" | "js" | "evm",
): Promise<StatsResult> => {
  const benchmarkRuns = getRuns(nruns);
  const results: Array<number> = [];

  let params = [];
  switch (type) {
    case "wasm":
      params = WASM_CMD;
      break;
    case "js":
      params = JS_CMD;
      break;
  }

  for (let i = 0; i < benchmarkRuns; i++) {
    results.push(
      await benchmark([
        ...params,
        ...extraParameters,
      ]),
    );
  }

  const [max, min, avg] = maxMinAvg(results);

  return {
    max,
    min,
    avg,
    runs: benchmarkRuns,
  } as StatsResult;
};
