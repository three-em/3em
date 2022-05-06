import {executeContract, executor, loadContract} from "./executor.js";
import { assertEquals } from "https://deno.land/std@0.123.0/testing/asserts.ts";

localStorage.clear();

Deno.test("contract_load_test", async () => {
  await loadContract("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE");
});

Deno.test("execute_js_test", async () => {
  const { validity } = await executeContract(
    "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE",
    749180,
  );
  assertEquals(Object.values(validity).filter((r) => !!r).length, 93);
});

Deno.test("execute_js_test", async () => {
  globalThis.ARWEAVE_HOST = "www.arweave.run";
  const { state } = await executeContract(
      "xRkYokQfFHLh2K9slmghlXNptKrqQdDZoy75JGsv89M",
      100,
  );
  assertEquals(state.ticker, "VRT");
});

Deno.test("execute_js_test readContractState", async () => {
  globalThis.ARWEAVE_HOST = "www.arweave.run";
  let { state, validity } = await executeContract(
      "Vjt13JlvOzaOs4St_Iy2jmanxa7dc-Z3pDk3ktwEQNA",
      undefined,
      false,
      undefined,
  );
  assertEquals(state.ticker, "CHILL");
});
