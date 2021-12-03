import { StatsResult } from "./model.ts";

export const benchmark = async (
  cmd: Array<string>,
  filePath: string,
): Promise<StatsResult> => {
  await Deno.run({
    cmd: [...cmd, "--show-output"],
    stdout: "null",
  }).status();

  return JSON.parse(Deno.readTextFileSync(filePath)).results[0] as StatsResult;
};
