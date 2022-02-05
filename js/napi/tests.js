const { executeContract } = require("./");

(async () => {
  const start = performance.now();
  const state1 = await executeContract(
    "t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE",
    749180,
  );
  const end = performance.now();
  console.log(state1);
  console.log(`[JS] Execution time: ${(end - start)}ms.`);

  const start2 = performance.now();
  const state2 = await executeContract(
    "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ",
    749180,
  );
  const end2 = performance.now();
  console.log(state2);
  console.log(`[WASM] Execution time: ${(end2 - start2)}ms.`);
})();