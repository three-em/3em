import { benchmark } from "./util.ts";
import {
  BenchmarkOptions,
  ContractTypes,
  Providers,
  StatsResult,
} from "./model.ts";
import {
  EM3_EVM_CMD,
  EM3_JS_CMD,
  EM3_WASM_CMD,
  REDSTONE_JS_CMD,
  SMARTWEAVE_JS_CMD,
} from "./commands.ts";

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
      let executor: any;
      switch (config.provider) {
        case Providers.EM3:
          executor = EM3_JS_CMD;
          break;
        case Providers.SMARTWEAVE:
          executor = SMARTWEAVE_JS_CMD;
          break;
      }
      params = executor(benchmarkRuns, config.file);
      break;
    case ContractTypes.EVM:
      params = EM3_EVM_CMD(benchmarkRuns, config.file);
  }

  results = await benchmark([
    ...params,
  ], config.file);

  return results;
};
