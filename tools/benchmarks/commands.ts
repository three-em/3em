const buildCommand = (
  command: Array<string>,
  runs: number,
  filePath: string,
) => {
  return [
    "hyperfine",
    `${command.join(" ")}`,
    "--min-runs",
    runs,
    "--export-json",
    filePath,
  ];
};

export const EM3_WASM_CMD = (runs: number, filePath: string) => {
  return buildCommand(
    [
      "target/release/bench_wasm",
      // KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ
    ],
    runs,
    filePath,
  );
};

export const EM3_EVM_CMD = (runs: number, filePath: string) => {
  return buildCommand(
    [
      "target/release/bench_evm",
    ],
    runs,
    filePath,
  );
};

export const EM3_JS_CMD = (runs: number, filePath: string) => {
  return buildCommand(
    [
      "target/release/bench",
    ],
    runs,
    filePath,
  );
};

export const SMARTWEAVE_JS_CMD = (runs: number, filePath: string) => {
  return buildCommand(
    [
      "node",
      "tools/benchmarks/smartweave/index.js",
    ],
    runs,
    filePath,
  );
};

