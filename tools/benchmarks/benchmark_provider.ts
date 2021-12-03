import { benchmark } from "./util.ts";
import { BenchmarkOptions, ContractTypes, StatsResult } from "./model.ts";
import { EM3_JS_CMD, EM3_WASM_CMD } from "./commands.ts";

const getRuns = (defaultRuns?: number): number => {
  const benchmarkRuns = Deno.env.get("RUNS");
  return defaultRuns || (benchmarkRuns ? parseInt(benchmarkRuns) : undefined) ||
    10;
};

export const benchmark3em = async (
  config: BenchmarkOptions,
): Promise<StatsResult> => {
  const benchmarkRuns = getRuns(config.runs);
  let results: StatsResult;

  let params = [];

  switch (config.type) {
    case ContractTypes.WASM:
      params = EM3_WASM_CMD(benchmarkRuns, config.file);
      break;
    case ContractTypes.JS:
      params = EM3_JS_CMD(benchmarkRuns, config.file);
      break;
  }

  results = await benchmark([
    ...params,
  ], config.file);

  return results;
};
