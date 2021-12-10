import { WasmRuntime } from "./index.js";

const encode = (s: any): Uint8Array =>
  new TextEncoder().encode(JSON.stringify(s));

Deno.test("testRustContract", async () => {
  const module = await Deno.readFile(
    "./helpers/rust/example/contract.wasm",
  );

  const rt = new WasmRuntime(
    module,
    {},
  );

  let curr_state = encode({ counter: 0 });
  const action = encode({});
  for (let i = 0; i < 100; i++) {
    const state = rt.call(new Uint8Array(curr_state), action);

    curr_state = state;
  }
});
