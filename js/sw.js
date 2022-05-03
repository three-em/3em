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

  const promises = [];
  class Contracts {
    readContractState(contractId, height, showValidity) {
      return new Promise((resolve, reject) => {
        const promiseId = Object.keys(globalThis.fcpCalls).length;
        globalThis.fcpCalls[promiseId] = {
          resolve,
          reject
        };
        self.postMessage({
          type: "fcp",
          contractId,
          height,
          showValidity,
          promiseId,
        });
      });
    }
  }
  
class ArweaveUtils {
    concatBuffers(
      buffers,
    ) {
      let total_length = 0;

      for (let i = 0; i < buffers.length; i++) {
        total_length += buffers[i].byteLength;
      }

      let temp = new Uint8Array(total_length);
      let offset = 0;

      temp.set(new Uint8Array(buffers[0]), offset);
      offset += buffers[0].byteLength;

      for (let i = 1; i < buffers.length; i++) {
        temp.set(new Uint8Array(buffers[i]), offset);
        offset += buffers[i].byteLength;
      }

      return temp;
    }

    b64UrlToString(b64UrlString) {
      let buffer = b64UrlToBuffer(b64UrlString);
      return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
    }

    bufferToString(buffer) {
      return new TextDecoder("utf-8", { fatal: true }).decode(buffer);
    }

    stringToBuffer(string) {
      return new TextEncoder().encode(string);
    }

    stringToB64Url(string) {
      return this.bufferTob64Url(stringToBuffer(string));
    }

    b64UrlToBuffer(b64UrlString) {
      return Uint8Array.from(atob(b64UrlString), (c) => c.charCodeAt(0));
    }

    bufferTob64(buffer) {
      return btoa(String.fromCharCode.apply(null, new Uint8Array(buffer)));
    }

    bufferTob64Url(buffer) {
      return this.b64UrlEncode(this.bufferTob64(buffer));
    }

    b64UrlEncode(b64UrlString) {
      return b64UrlString
        .replace(/\\+/g, "-")
        .replace(/\\//g, "_")
        .replace(/\\=/g, "");
    }

    b64UrlDecode(b64UrlString) {
      b64UrlString = b64UrlString.replace(/\\-/g, "+").replace(/\\_/g, "/");
      let padding = 0;
      if (b64UrlString.length % 4 !== 0) {
        padding = 4 - (b64UrlString.length % 4);
      }

      return b64UrlString.concat("=".repeat(padding));
    }
  }
  
  class BaseObject {
    constructor() {
      this.arweaveUtils = new ArweaveUtils();
    }

    get(field, options) {
      // Handle fields that are Uint8Arrays.
      // To maintain compat we encode them to b64url
      // if decode option is not specificed.
      if (this[field] instanceof Uint8Array) {
          if (options && options.decode && options.string) {
            return new TextDecoder().decode(this[field]);
          }
          if (options && options.decode && !options.string) {
            return this[field];
          }
          return this.arweaveUtils.bufferTob64Url(this[field]);
      }
    }
  }

  class Tag extends BaseObject {
    constructor(name, value) {
      super();
      this.name = name;
      this.value = value;
    }
  }

  class Transaction extends BaseObject {
    constructor(obj) {
      super();
      Object.assign(this, obj);

      if (typeof this.data === "string") {
        this.data = this.arweaveUtils.b64UrlToString(this.data);
      }

      if (obj.tags) {
        this.tags = obj.tags.map(t => new Tag(t.name, t.value));
      }
    }
  }

  class UnsafeClientTransactions {
    constructor(obj) {
      this.arweaveUtils = new ArweaveUtils();
    }
    
    async get(txId, opts) {
       const resp = await fetch("${globalThis.URL_GATEWAY || 'https://arweave.net'}" + "/tx/" + txId);
       const json = await resp.json();
       if (data.status === 200) {
         const data = await this.getData(id);
         return new Transaction({
              ...json,
              data,
         });
       }
    }
    
    async getData(txId) {
       const resp = await fetch("${globalThis.URL_GATEWAY || 'https://arweave.net'}" + "/tx/" + txId);
       const data = new Uint8Array(await resp.arrayBuffer());
       if (opts && opts.decode && !opts.string) {
          return data;
       }
       if (opts && opts.decode && opts.string) {
          return this.arweaveUtils.bufferToString(data);
       }
       return this.arweaveUtils.bufferTob64Url(data);
    }
  }

  class SmartWeave {
    get contracts() {
      return new Contracts();
    }

    get transaction() {
      return globalThis.interactionContext.transaction;
    }
    
    get block() {
      const block = globalThis.interactionContext.block;
      return {...block, indep_hash: block.id };
    }    

    get unsafeClient() {
      return {
        transactions: new UnsafeClientTransactions()
      }
    }
  }
  
  function handleInteractionGlobals(tx) { 
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
      }
    }
  }
  
  globalThis.ContractError = ContractError;
  globalThis.ContractAssert = ContractAssert;
  globalThis.fcpCalls = {};
  globalThis.SmartWeave = new SmartWeave();
  globalThis.evaluateInteractions = async (currentState, interactions, input) => {
    const validity = {};
    if (interactions.length == 0) {
        try {
          const state = await handle(
            currentState,
            { input },
          );
  
          currentState = state.state;
        } catch(e) {}
    }
    for (let i = 0; i < interactions.length; i++) {
        const tx = interactions[i].node;
        handleInteractionGlobals(tx);
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

    return { state: currentState, validity };
  };
  self.addEventListener("message", async function(e) {
    if(e.data.type === "fcp") {
        await ((async function() {
          const { state: s, source, promiseId } = e.data;
          let currentState = typeof s == "string" ? JSON.parse(s) : s;
          const interactions = e.data.interactions ?? [];
          if(source) eval(source);
          const fcpState = await globalThis.evaluateInteractions(currentState, interactions, e.data.action);
          globalThis.fcpCalls[promiseId].resolve(fcpState);
        })());
    }
    if(e.data.type === "execute") {
      let currentState = JSON.parse(e.data.state);
      const interactions = e.data.interactions ?? [];
      const contractState = await globalThis.evaluateInteractions(currentState, interactions, e.data.action);
      self.postMessage(contractState);
    }
  });
}`;

export class Runtime {
  #state;
  #module;
  #fcpHandler;

  constructor(source, state = {}, info = {}, fcpHandler, urlGateway) {
    this.#state = state;
    const setUrlGateway = `
      globalThis.URL_GATEWAY = "${urlGateway}";
    `
    const sources = [WORKER, source, setUrlGateway];
    const blob = new Blob(sources, { type: "application/javascript" });
    this.#module = new Worker(
      URL.createObjectURL(blob),
      { eval: true, type: "module" },
    );
    this.#fcpHandler = fcpHandler;
  }

  async resolveState() {
    this.#state = await new Promise((resolve) => {
      this.#module.onmessage = async (e) => {
        if (e.data?.type === "fcp") {
          const { promiseId, contractId, height, showValidity } = e.data;
          const [contract, interactions, updatePromise] = await this.#fcpHandler(contractId, height, showValidity);
          const { source, state } = contract;
          this.#module.postMessage({
            type: "fcp",
            promiseId,
            source,
            state,
            interactions: [...interactions, ...updatePromise],
          });
          return;
        }
        resolve(e.data);
        this.#module.terminate();
      };
    });
  }

  // Fast path for the most common case.
  async executeInteractions(interactions) {
    this.#module.postMessage({
      type: "execute",
      state: this.#state,
      interactions,
    });

    await this.resolveState();
  }

  async execute(action = {}) {
    this.#module.postMessage({
      type: "execute",
      state: this.#state,
      action,
      interactions: [],
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
