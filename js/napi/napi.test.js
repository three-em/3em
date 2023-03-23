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

  test("Simulate contract, errors", async () => {
    const buffer = new TextEncoder().encode(`
export async function handle(state, action) {
    state.counts++;
    let stateObj = { state };
    if(state.counts > 1) {
        throw new Error("Ups");
    }
    return stateObj;
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
      },{
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
            counts: 0
          }
      )
    });
   expect(simulate.errors["ABCD"]).toContain("Ups")
  });

  test("Deterministic fetch lazy evaluation", async () => {
    const buffer = new TextEncoder().encode("export async function handle(state, action) {\n" +
        "    const ethTxId = action.input.id;\n" +
        "    const fetchTx = await EXM.deterministicFetch(`https://api.blockcypher.com/v1/eth/main/txs/${ethTxId}`);\n" +
        "EXM.print(fetchTx);" +
        "    const txJson = fetchTx.asJSON();\n" +
        "    state[ethTxId] = txJson.fees;\n" +
        "    return {\n" +
        "        state,\n" +
        "        result: txJson\n" +
        "    }\n" +
        "}");

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
        input: JSON.stringify({"id":"8f39fb4940c084460da00a876a521ef2ba84ad6ea8d2f5628c9f1f8aeb395342"})
      }],
      contractInitState: JSON.stringify({}),
      maybeSettings: {
        'LAZY_EVALUATION': true
      },
      maybeExmContext: `{"requests":{"d1b57dac5f734a8831b808bdd5dfdb9cd0d12a56014190982ce744d99a9a661c":{"type":"basic","url":"https://api.blockcypher.com/v1/eth/main/txs/8f39fb4940c084460da00a876a521ef2ba84ad6ea8d2f5628c9f1f8aeb395342","statusText":"OK","status":127,"redirected":false,"ok":true,"headers":{"x-ratelimit-remaining":"99","access-control-allow-methods":"GET, POST, PUT, DELETE","server":"cloudflare","cf-cache-status":"DYNAMIC","access-control-allow-origin":"*","content-type":"application/json","date":"Tue, 21 Mar 2023 18:14:43 GMT","access-control-allow-headers":"Origin, X-Requested-With, Content-Type, Accept","cf-ray":"7ab82d1aed372937-ORD-X"},"vector":[123,10,32,32,34,98,108,111,99,107,95,104,97,115,104,34,58,32,34,98,51,53,50,54,53,57,54,53,102,54,102,49,52,99,53,100,54,57,50,98,50,48,51,48,57,50,50,102,99,101,53,51,48,54,55,102,99,48,101,57,100,56,99,56,52,57,48,99,51,49,100,52,101,99,100,51,101,52,54,53,48,48,99,34,44,10,32,32,34,98,108,111,99,107,95,104,101,105,103,104,116,34,58,32,49,53,54,52,48,52,48,44,10,32,32,34,98,108,111,99,107,95,105,110,100,101,120,34,58,32,48,44,10,32,32,34,104,97,115,104,34,58,32,34,56,102,51,57,102,98,52,57,52,48,99,48,56,52,52,54,48,100,97,48,48,97,56,55,54,97,53,50,49,101,102,50,98,97,56,52,97,100,54,101,97,56,100,50,102,53,54,50,56,99,57,102,49,102,56,97,101,98,51,57,53,51,52,50,34,44,10,32,32,34,97,100,100,114,101,115,115,101,115,34,58,32,91,10,32,32,32,32,34,52,101,57,100,56,98,52,102,49,56,100,57,56,52,102,54,102,48,99,56,56,100,48,55,101,52,98,51,57,50,48,49,101,56,50,53,99,100,49,55,34,44,10,32,32,32,32,34,55,51,56,100,49,52,53,102,97,97,98,98,49,101,48,48,99,102,53,97,48,49,55,53,56,56,97,57,99,48,102,57,57,56,51,49,56,48,49,50,34,10,32,32,93,44,10,32,32,34,116,111,116,97,108,34,58,32,49,48,49,53,51,49,53,51,51,53,57,52,51,55,51,50,54,44,10,32,32,34,102,101,101,115,34,58,32,49,53,57,53,53,56,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,115,105,122,101,34,58,32,49,49,54,44,10,32,32,34,103,97,115,95,108,105,109,105,116,34,58,32,53,48,48,48,48,48,44,10,32,32,34,103,97,115,95,117,115,101,100,34,58,32,55,57,55,55,57,44,10,32,32,34,103,97,115,95,112,114,105,99,101,34,58,32,50,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,103,97,115,95,116,105,112,95,99,97,112,34,58,32,50,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,103,97,115,95,102,101,101,95,99,97,112,34,58,32,50,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,99,111,110,102,105,114,109,101,100,34,58,32,34,50,48,49,54,45,48,53,45,50,50,84,49,50,58,52,51,58,48,48,90,34,44,10,32,32,34,114,101,99,101,105,118,101,100,34,58,32,34,50,48,49,54,45,48,53,45,50,50,84,49,50,58,52,51,58,48,48,90,34,44,10,32,32,34,118,101,114,34,58,32,48,44,10,32,32,34,100,111,117,98,108,101,95,115,112,101,110,100,34,58,32,102,97,108,115,101,44,10,32,32,34,118,105,110,95,115,122,34,58,32,49,44,10,32,32,34,118,111,117,116,95,115,122,34,58,32,49,44,10,32,32,34,105,110,116,101,114,110,97,108,95,116,120,105,100,115,34,58,32,91,10,32,32,32,32,34,100,100,49,48,55,99,56,52,56,56,56,54,55,102,100,53,51,99,48,97,97,51,98,102,49,100,56,97,52,55,56,97,48,55,55,101,99,54,55,97,102,55,53,56,52,50,100,50,52,102,49,97,54,52,101,98,52,52,101,52,100,57,48,50,34,44,10,32,32,32,32,34,57,97,57,56,54,55,56,100,50,48,57,57,49,48,102,55,48,98,100,101,100,54,50,52,102,50,99,53,99,49,56,101,100,49,55,100,52,49,55,55,53,50,48,55,50,100,52,49,99,53,100,54,49,98,52,50,99,50,55,55,51,52,56,99,34,44,10,32,32,32,32,34,97,100,57,53,57,57,54,49,102,52,54,54,54,51,102,55,56,53,101,49,101,48,97,48,98,50,54,53,54,57,54,48,98,53,100,50,55,48,54,49,48,57,55,99,53,48,57,50,99,52,101,54,56,57,54,52,98,102,97,102,48,48,52,49,34,44,10,32,32,32,32,34,53,97,50,51,100,55,52,97,52,56,52,101,99,53,56,98,50,49,49,50,54,52,56,56,56,56,52,54,98,101,54,100,55,52,50,56,99,51,51,98,101,57,51,50,50,51,51,57,54,101,102,102,51,54,50,54,101,48,51,54,97,55,102,52,34,44,10,32,32,32,32,34,55,51,57,48,54,98,50,102,49,97,49,100,55,102,100,49,55,99,55,55,54,56,52,101,53,101,56,49,49,97,101,56,55,49,48,101,52,97,48,51,50,53,48,49,48,57,48,100,97,50,54,52,55,49,50,100,98,55,97,52,56,101,55,97,34,44,10,32,32,32,32,34,53,51,99,101,56,56,101,49,99,102,57,56,98,51,55,102,98,54,52,99,49,54,49,50,49,98,52,54,52,49,100,100,101,54,53,50,57,48,56,51,56,99,101,48,98,100,101,54,99,98,49,99,51,49,57,100,102,51,101,102,56,102,102,57,34,44,10,32,32,32,32,34,50,57,98,99,52,101,55,102,97,50,100,98,50,56,98,48,97,50,101,102,57,101,55,97,53,101,48,55,51,99,99,55,53,51,50,101,54,48,57,102,55,53,50,98,50,101,98,48,98,53,55,51,49,98,48,99,54,98,57,50,54,97,50,99,34,44,10,32,32,32,32,34,52,97,57,97,102,99,53,97,54,56,48,57,49,55,57,53,53,101,55,56,49,98,56,57,48,54,56,56,49,52,51,102,53,54,98,97,100,99,55,54,55,56,53,97,102,57,56,55,53,51,100,53,50,54,55,50,55,48,100,55,100,56,48,57,34,10,32,32,93,44,10,32,32,34,99,111,110,102,105,114,109,97,116,105,111,110,115,34,58,32,49,53,51,49,51,54,56,48,44,10,32,32,34,99,111,110,102,105,100,101,110,99,101,34,58,32,49,44,10,32,32,34,105,110,112,117,116,115,34,58,32,91,10,32,32,32,32,123,10,32,32,32,32,32,32,34,115,101,113,117,101,110,99,101,34,58,32,50,55,51,44,10,32,32,32,32,32,32,34,97,100,100,114,101,115,115,101,115,34,58,32,91,10,32,32,32,32,32,32,32,32,34,55,51,56,100,49,52,53,102,97,97,98,98,49,101,48,48,99,102,53,97,48,49,55,53,56,56,97,57,99,48,102,57,57,56,51,49,56,48,49,50,34,10,32,32,32,32,32,32,93,10,32,32,32,32,125,10,32,32,93,44,10,32,32,34,111,117,116,112,117,116,115,34,58,32,91,10,32,32,32,32,123,10,32,32,32,32,32,32,34,118,97,108,117,101,34,58,32,49,48,49,53,51,49,53,51,51,53,57,52,51,55,51,50,54,44,10,32,32,32,32,32,32,34,115,99,114,105,112,116,34,58,32,34,52,101,55,49,100,57,50,100,34,44,10,32,32,32,32,32,32,34,97,100,100,114,101,115,115,101,115,34,58,32,91,10,32,32,32,32,32,32,32,32,34,52,101,57,100,56,98,52,102,49,56,100,57,56,52,102,54,102,48,99,56,56,100,48,55,101,52,98,51,57,50,48,49,101,56,50,53,99,100,49,55,34,10,32,32,32,32,32,32,93,10,32,32,32,32,125,10,32,32,93,10,125]}}}`
    });
    expect(simulate.state).toEqual({"8f39fb4940c084460da00a876a521ef2ba84ad6ea8d2f5628c9f1f8aeb395342":1595580000000000});
  });

})
