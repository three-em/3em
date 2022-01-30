import { executeContract, loadContract } from "./executor";

Deno.test("contract_load_test", async function () {
  await loadContract("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE");
});

Deno.test("execute_js_test", async function () {
  await executeContract("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE", 749180);
});
