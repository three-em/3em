const { executeContract, simulateContract } = require("./");

describe("NAPI test", () => {

  jest.setTimeout(10000);

  test("Test contract", async () => {
    const run = await executeContract("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE");
    expect(run.state.tokens).not.toBeUndefined();
  })

  test("Simulate contract", async () => {
    const simulate = await simulateContract(
    "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ",
    [{
      id: "ABCD",
      owner: "2asdaskdsapdk012",
      quantity: "1000",
      reward: "203123921",
      target: "none",
      tags: [],
      input: {}
    }],
    JSON.stringify({
          counter: 9499
        }
    ));
    expect(simulate.state.counter).toBe(9500);
  })
})
