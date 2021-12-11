import { Runtime } from "./index.js";

Deno.test("test_runtime_1", async () => {
  const runtime = new Runtime(
    "export async function handle({ counter }) { return { state: counter + 1 } }",
    {
      counter: 0,
    },
    {},
  );

  await runtime.execute({});

  console.log(runtime.state);

  runtime.destroy();
});
