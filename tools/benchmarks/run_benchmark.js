import { runBenchmarks } from "./benchmark.ts";

const convertTZ = (date, tzString) => {
  return new Date(
    (typeof date === "string" ? new Date(date) : date).toLocaleString("en-US", {
      timeZone: tzString,
    }),
  );
};

const appendBenchmarks = async () => {
  const benchmarkPath = Deno.env.get("CI")
    ? "./data/benchmark.json"
    : "./data/benchmark_dev.json";
  const currentBenchmarks = JSON.parse(await Deno.readTextFile(benchmarkPath));
  const currentDate = convertTZ(new Date(), "America/New_York");
  const currentDateMs = currentDate.getTime();
  const benchmarkLength = currentBenchmarks.length;

  let newBenchmarks = [...currentBenchmarks];

  const newBenchmarkObj = {
    createdAt: currentDate.toString(),
    createdAtTime: currentDateMs,
    benchmark: {
      ...await runBenchmarks(),
    },
  };

  if (benchmarkLength < 30) {
    newBenchmarks = [newBenchmarkObj, ...newBenchmarks];
  } else {
    newBenchmarks = currentBenchmarks.sort((a, b) => {
      return a.createdAtTime - b.createdAtTime;
    });
    newBenchmarks.pop();
    newBenchmarks = [newBenchmarkObj, ...newBenchmarks];
  }

  await Deno.writeTextFile(
    benchmarkPath,
    JSON.stringify(newBenchmarks, null, 2),
  );

  await Deno.run({
    cmd: ["node", "generate_benchmark_img.js"],
    cwd: "./tools/benchmarks"
  }).status();
};

await appendBenchmarks();
