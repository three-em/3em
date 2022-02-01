import { Runtime } from "./sw.js";
import { assertEquals } from "https://deno.land/std@0.123.0/testing/asserts.ts";

Deno.test("test_runtime_1", async () => {
  const runtime = new Runtime(
    "export function handle(state) { state.counter += 1; return { state } }",
    {
      counter: 0,
    },
  );

  await runtime.execute({});

  assertEquals((runtime.state as any).counter, 1);
  runtime.destroy();
});

Deno.test("test_runtime_deterministic", async () => {
  const runtime = new Runtime(
    `export async function handle() {
  return { 
    state: {
      performance_now: performance.now(),
      random: Math.random(),
      date: new Date(),
      date_now: Date.now(),
    }
  }
}`,
  );

  await runtime.execute();

  const state = runtime.state as any;
  assertEquals(state.performance_now, 0);
  assertEquals(state.random, 0.3800000002095474);
  assertEquals(state.date, new Date("2016-11-18T00:00:00.000Z"));
  assertEquals(state.date_now, 1479427200000);

  runtime.destroy();
});
