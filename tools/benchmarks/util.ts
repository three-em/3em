export type MilliSeconds = number;

export const WASM_CMD = [
  "target/release/three_em",
  "run",
  "--contract-id",
  "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ",
];

export const JS_CMD = [
  "target/release/three_em",
  "run",
  "--contract-id",
  "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE",
  "--height",
  "749180",
];

export const benchmark = async (cmd: Array<string>): Promise<MilliSeconds> => {
  const start = performance.now();
  await Deno.run({
    cmd,
    stdout: "null",
  }).status();
  let end = performance.now();
  return end - start;
};

export const maxMinAvg = (arr: Array<number>): [number, number, number] => {
  let max = arr[0];
  let min = arr[0];
  let sum = arr[0];
  for (let i = 1; i < arr.length; i++) {
    if (arr[i] > max) {
      max = arr[i];
    }
    if (arr[i] < min) {
      min = arr[i];
    }
    sum = sum + arr[i];
  }
  return [max, min, sum / arr.length];
};
