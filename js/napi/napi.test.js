const { executeContract, simulateContract } = require("./");
const {SimulateContractType} = require("./index");

describe("NAPI test", () => {

  jest.setTimeout(25000);

  test("Test contract", async () => {
    const run = await executeContract("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE");
    expect(run.state.tokens).not.toBeUndefined();
  })

  test("Failed simulation", async () => {
    try {
      const simulate = await simulateContract({
              contractId: "invalidId",
              interactions: [{
                id: "ABCD",
                owner: "2asdaskdsapdk012",
                quantity: "1000",
                reward: "203123921",
                target: "none",
                tags: [],
                input: JSON.stringify({})
              }],
              contractInitState: JSON.stringify({
                    counter: 9499
              })
      });
      expect(false).toBeTruthy();
    } catch (e) {
      console.log('Caught error', e.toString())
    }
  })

  test("Simulate contract", async () => {
    const simulate = await simulateContract({
    contractId: "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ",
    interactions: [{
      id: "ABCD",
      owner: "2asdaskdsapdk012",
      quantity: "1000",
      reward: "203123921",
      target: "none",
      tags: [],
      input: JSON.stringify({})
    }],
    contractInitState: JSON.stringify({
          counter: 9499
        }
    )});
    expect(simulate.state.counter).toBe(9500);
  });

  test("Simulate contract, manual source", async () => {
    const buffer = new TextEncoder().encode(`
    /**
 *
 * @param state is the current state your application holds
 * @param action is an object containing { input, caller } . Most of the times you will only use \`action.input\` which contains the input passed as a write operation
 * @returns {Promise<{ users: Array<{ username: string}> }>}
 */
export async function handle(state, action) {
    const { username } = action.input;
    state.users.push({ username });
    return { state, result: 'Hello World' };
}
    `);

    const simulate = await simulateContract({
      contractId: "",
      maybeContractSource: {
        contractType: SimulateContractType.JAVASCRIPT,
        contractSrc: buffer
      },
      interactions: [{
        id: "ABCD",
        owner: "2asdaskdsapdk012",
        quantity: "1000",
        reward: "203123921",
        target: "none",
        tags: [],
        input: JSON.stringify({
          username: "Andres"
        })
      }],
      contractInitState: JSON.stringify({
            users: []
          }
      )
    });
    expect(simulate.state.users).toEqual([{ username: "Andres" }]);
    expect(simulate.result).toEqual("Hello World");
    expect(simulate.updated).toBeTruthy();
  });

  test("Simulate contract, manual source, not state just result", async () => {
    const buffer = new TextEncoder().encode(`
    /**
 *
 * @param state is the current state your application holds
 * @param action is an object containing { input, caller } . Most of the times you will only use \`action.input\` which contains the input passed as a write operation
 * @returns {Promise<{ users: Array<{ username: string}> }>}
 */
export async function handle(state, action) {
    return { result: 'Hello World' };
}
    `);

    const simulate = await simulateContract({
      contractId: "",
      maybeContractSource: {
        contractType: SimulateContractType.JAVASCRIPT,
        contractSrc: buffer
      },
      interactions: [{
        id: "ABCD",
        owner: "2asdaskdsapdk012",
        quantity: "1000",
        reward: "203123921",
        target: "none",
        tags: [],
        input: JSON.stringify({
          username: "Andres"
        })
      }],
      contractInitState: JSON.stringify({
            users: []
          }
      )
    });
    expect(simulate.result).toEqual("Hello World");
    expect(simulate.updated).toBeFalsy();
  });
})
