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
      "target/release/three_em",
      "run",
      "--contract-id",
      "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ",
    ],
    runs,
    filePath,
  );
};

export const EM3_JS_CMD = (runs: number, filePath: string) => {
  return buildCommand(
    [
      "target/release/three_em",
      "run",
      "--contract-id",
      "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE",
      "--height",
      "749180",
    ],
    runs,
    filePath,
  );
};
