export enum ContractTypes {
  WASM,
  JS,
  EVM,
}

export enum Providers {
  EM3,
  REDSTONE,
  SMARTWEAVE,
}

export interface BenchmarkOptions {
  file: string;
  type: ContractTypes;
  provider: Providers;
  runs?: number;
}

export interface StatsResult {
  mean: number;
  stddev: number;
  median: number;
  user: number;
  system: number;
  min: number;
  max: number;
  times: Array<number>;
}
