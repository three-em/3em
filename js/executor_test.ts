import { executeContract, loadContract } from "./executor.js";
import { assertEquals } from "https://deno.land/std@0.123.0/testing/asserts.ts";

Deno.test("contract_load_test", async function () {
  await loadContract("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE");
});

Deno.test("execute_js_test", async function () {
  const { validity } = await executeContract(
    "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE",
    749180,
  );
  assertEquals(Object.values(validity).filter((r) => !!r).length, 93);
});

Deno.test("execute_js_test", async function () {
  // @ts-ignore
  globalThis.ARWEAVE_HOST = "www.arweave.run";
  const { state } = await executeContract(
      "xRkYokQfFHLh2K9slmghlXNptKrqQdDZoy75JGsv89M",
      undefined,
  );
  assertEquals(state.ticker, "VRT");
});
