const WORKER = `{
  const selfCloned = globalThis;
  const allowed = ["Reflect", "Event", "performance", "ErrorEvent", "self", "MessageEvent", "postMessage", "addEventListener"];
  const keys = Object.keys(globalThis).filter(key => !allowed.includes(key));
  for (const key of keys) {
    Reflect.deleteProperty(globalThis, key);
  }

  // Remove non-deterministic GC dependent V8 globals.
  FinalizationRegistry = class FinalizationRegistry {
    #register;
    constructor(fn) {
      this.#register = fn;
    }

    register() { /* Nop */ }
  }

  WeakRef = class WeakRef {
    #value;
    constructor(value) { this.#value = value; }

    deref() {
      return this.#value;
    }
  };

  // xorshift128+ RNG adapted from https://github.com/AndreasMadsen/xorshift
  const s = 0.69 * Math.pow(2, 32);
  const seed = [
    s, s, s, s
  ];
  // uint64_t s = [seed ...]
  let _state0U = seed[0] | 0;
  let _state0L = seed[1] | 0;
  let _state1U = seed[2] | 0;
  let _state1L = seed[3] | 0;

  Math.random = function() {
    // uint64_t s1 = s[0]
    var s1U = _state0U, s1L = _state0L;
    // uint64_t s0 = s[1]
    var s0U = _state1U, s0L = _state1L;
  
    // result = s0 + s1
    var sumL = (s0L >>> 0) + (s1L >>> 0);
    var resU = (s0U + s1U + (sumL / 2 >>> 31)) >>> 0;
    var resL = sumL >>> 0;
  
    // s[0] = s0
    _state0U = s0U;
    _state0L = s0L;
  
    // - t1 = [0, 0]
    var t1U = 0, t1L = 0;
    // - t2 = [0, 0]
    var t2U = 0, t2L = 0;
  
    // s1 ^= s1 << 23;
    // :: t1 = s1 << 23
    var a1 = 23;
    var m1 = 0xFFFFFFFF << (32 - a1);
    t1U = (s1U << a1) | ((s1L & m1) >>> (32 - a1));
    t1L = s1L << a1;
    // :: s1 = s1 ^ t1
    s1U = s1U ^ t1U;
    s1L = s1L ^ t1L;
  
    // t1 = ( s1 ^ s0 ^ ( s1 >> 17 ) ^ ( s0 >> 26 ) )
    // :: t1 = s1 ^ s0
    t1U = s1U ^ s0U;
    t1L = s1L ^ s0L;
    // :: t2 = s1 >> 18
    var a2 = 18;
    var m2 = 0xFFFFFFFF >>> (32 - a2);
    t2U = s1U >>> a2;
    t2L = (s1L >>> a2) | ((s1U & m2) << (32 - a2));
    // :: t1 = t1 ^ t2
    t1U = t1U ^ t2U;
    t1L = t1L ^ t2L;
    // :: t2 = s0 >> 5
    var a3 = 5;
    var m3 = 0xFFFFFFFF >>> (32 - a3);
    t2U = s0U >>> a3;
    t2L = (s0L >>> a3) | ((s0U & m3) << (32 - a3));
    // :: t1 = t1 ^ t2
    t1U = t1U ^ t2U;
    t1L = t1L ^ t2L;
  
    // s[1] = t1
    _state1U = t1U;
    _state1L = t1L;

    return resU * 2.3283064365386963e-10 + (resL >>> 12) * 2.220446049250313e-16;
  }

  const clonedDate = Date;
  function NewDate(...args) {
    const dateArgs = args.length === 0 ? [1479427200000] : args;
    const instance = new clonedDate(...dateArgs);
    Object.setPrototypeOf(instance, Object.getPrototypeOf(NewDate.prototype));
    return instance;
  }

  NewDate.prototype = Object.create(Date.prototype);
  Object.setPrototypeOf(NewDate, Date);

  NewDate.now = () => 1479427200000; // 2016-11-18 00:00:00.000
  
  Date = NewDate;

  let step = 0.0;
  performance.now = () => {
    const now = step;
    step += 0.1;
    return now;
  }

  // JSON.stringify is deterministic. No action required there.
  // https://github.com/nodejs/node/issues/15628#issuecomment-332588533
  
  class ContractError extends Error {
    constructor(message) {
      super(message);
      this.name = "ContractError";
    }
  }

  function ContractAssert(cond, message) {
    if (!cond) throw new ContractError(message);
  }
  
  class SmartWeave {
    
    constructor() {
      this.readContractCalls = {};
      this.k = 0;
    }
  
    get transaction() {
      return globalThis.interactionContext.transaction;
    }
    
    get block() {
      const block = globalThis.interactionContext.block;
      return {...block, indep_hash: block.id };
    }
    
    get contract() { 
      return {
        id: globalThis.interactionContext.contract.id,
        owner: globalThis.interactionContext.contract.owner
      };
    }
    
    get contracts() {
      return { 
        readContractState: async (contractId) => {
          const key = this.k++;
          self.postMessage({
            readContractState: true,
            contractId,
            key
          });
          
          return new Promise((r) => {
            this.readContractCalls[key] = r;
          });
        }
      }
    }   
  }
  
  function handleInteractionGlobals(tx, contract) { 
    globalThis.interactionContext = { 
      transaction: {
        id: tx.id,
        owner: tx.owner.address,
        tags: [...(tx.tags)],
        target: tx.recipient,
        quantity: tx.quantity,
        reward: tx.fee
      },
      block: {
        height: tx.block.height,
        id: tx.block.id,
        timestamp: tx.block.timestamp
      },
      contract
    }
  }
  
  globalThis.ContractError = ContractError;
  globalThis.ContractAssert = ContractAssert;
  globalThis.SmartWeave = new SmartWeave();
  self.addEventListener("message", async function(e) {
    if(e.data.type === "execute") {
      let currentState = JSON.parse(e.data.state);
      const interactions = e.data.interactions ?? [];
      
      if (interactions.length == 0) {
        const input = e.data.action;
        try {
          const state = await handle(
            currentState,
            { input },
          );
  
          currentState = state.state;
        } catch(e) {}
      }

      const validity = {};
      for (let i = 0; i < interactions.length; i++) {
        const tx = interactions[i].node;
        handleInteractionGlobals(tx, e.data.contract);
        const input = tx.tags.find(data => data.name === "Input");

        try {
          const inp = JSON.parse(input.value);
          const state = await handle(
            currentState,
            { tx, input: inp, caller: tx.owner.address },
          );
          if (!state) {
            validity[tx.id] = false;
            continue;
          }
          currentState = state.state;
          validity[tx.id] = true;
        } catch(e) {
          validity[tx.id] = false;
        }
      }

      self.postMessage({ state: currentState, validity });
    }
    
    if(e.data.type === "readContractState") {
      const { state, key, contractId } = e.data;
      globalThis.SmartWeave.readContractCalls[key](state);
    }
  });
}`;

export class Runtime {
  #state;
  #module;

  constructor(source, state = {}, info = {}, executor, gateway) {
    this.#state = state;
    const sources = [WORKER, source];
    const blob = new Blob(sources, { type: "application/javascript" });
    this.#module = new Worker(
      URL.createObjectURL(blob),
      { eval: true, type: "module" },
    );
    this.executor = executor;
    this.gateway = gateway;
  }

  async resolveState() {
    this.#state = await new Promise((resolve) => {
      this.#module.onmessage = async (e) => {
        if(e.data.readContractState) {
          const { contractId, key } = e.data;
          const { state } = await this.executor.executeContract(contractId, undefined, undefined, this.gateway);
          this.#module.postMessage({
            type: "readContractState",
            state,
            key,
            contractId
          })
        } else {
          resolve(e.data);
          this.#module.terminate();
        }
      };
    });
  }

  // Fast path for the most common case.
  async executeInteractions(interactions, contract) {
    this.#module.postMessage({
      type: "execute",
      state: this.#state,
      interactions,
      contract
    });

    await this.resolveState();
  }

  async execute(action = {}, contract) {
    this.#module.postMessage({
      type: "execute",
      state: this.#state,
      action,
      interactions: [],
      contract
    });

    await this.resolveState();
  }

  get state() {
    return this.#state;
  }

  destroy() {
    this.#module.terminate();
  }
}
