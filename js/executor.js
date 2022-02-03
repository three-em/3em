import { Runtime } from "./sw.js";
import { hex, Machine } from "./evm/index.js";
import { WasmRuntime } from "./wasm.js";

const isNode = typeof process == "object";
if (isNode) {
  const w = await import("worker_threads");
  global.Worker = w.Worker;
  const L = (await import("node-localstorage")).LocalStorage;
  const findCacheDir = (await import("find-cache-dir")).default;
  global.localStorage = new L(findCacheDir({name: '3em'}));
}

const getTagSource = `
function getTag(tx, field) {
  const encodedName = btoa(field);
  return atob(tx.tags.find((data) => data.name === encodedName)?.value || "");
}`;

const loadContractSource = `
const baseUrl = "https://arweave.net";
const isNode = typeof process == "object";
const me = isNode ? require("worker_threads").parentPort : self;
if(isNode) {
  const fetch = (...args) => import('node-fetch').then(({default: fetch}) => fetch(...args));
  global.fetch = fetch;
}
async function loadContract(contractId) {
  const response = await fetch(new URL(\`/tx/\${contractId}\`, baseUrl).href);
  const tx = await response.json();

  const contractSrcTxID = getTag(tx, "Contract-Src");

  const contractSrcResponse = await fetch(
    new URL(\`/tx/\${contractSrcTxID}/data\`, baseUrl).href,
  );
  const contractSrc = await contractSrcResponse.text();
  
  const source = atob(contractSrc.replace(/_/g, "/").replace(/-/g, "+"));
  let state = getTag(tx, "Init-State");

  if (!state) {
    const stateTx = getTag(tx, "Init-State-TX");
    if (stateTx) {
      const stateTxResponse = await fetch(
        new URL(\`/tx/\${stateTx}/data\`, baseUrl).href,
      );
      const stateTxData = await stateTxResponse.text();
      state = atob(stateTxData.data.replace(/_/g, "/").replace(/-/g, "+"));
    } else {
      const txDataResonse = await fetch(
        new URL(\`/tx/\${contractId}/data\`, baseUrl).href,
      );
      const txData = await txDataResonse.text();

      state = atob(txData.replace(/_/g, "/").replace(/-/g, "+"));
    }
  }
  const sourceTxResponse = await fetch(
    new URL(\`/tx/\${contractSrcTxID}\`, baseUrl).href,
  );
  const sourceTx = await sourceTxResponse.json();
  const type = getTag(sourceTx, "Content-Type");

  return {
    source,
    state,
    type,
  };
}

me.addEventListener("message", (event) => {
  const { tx } = event.data;
  loadContract(tx).then((r) => me.postMessage(r));
});
`;

const loadContractSources = [getTagSource, loadContractSource];
const loadContractBlob = isNode
  ? loadContractSources.join("\n")
  : new Blob(loadContractSources, {
    type: "application/javascript",
  });
const loadContractWorker = new Worker(
  isNode ? loadContractBlob : URL.createObjectURL(loadContractBlob),
  { eval: true, type: "module" },
);

const loadInteractionsSource = `
const baseUrl = "https://arweave.net";
const isNode = typeof process == "object";
const me = isNode ? require("worker_threads").parentPort : self;
if(isNode) {
  const fetch = (...args) => import('node-fetch').then(({default: fetch}) => fetch(...args));
  global.fetch = fetch;
}
const query =
  \`query Transactions($tags: [TagFilter!]!, $blockFilter: BlockFilter!, $first: Int!, $after: String) {
  transactions(tags: $tags, block: $blockFilter, first: $first, sort: HEIGHT_ASC, after: $after) {
    pageInfo {
      hasNextPage
    }
    edges {
      node {
        id
        owner { address }
        recipient
        tags {
          name
          value
        }
        block {
          height
          id
          timestamp
        }
        fee { winston }
        quantity { winston }
        parent { id }
      }
      cursor
    }
  }
}\`;

const MAX_REQUEST = 200;

async function nextPage(variables) {
  const response = await fetch(
    new URL("/graphql", baseUrl).href,
    {
      method: "POST",
      body: JSON.stringify({
        query,
        variables,
      }),
      headers: {
        "Content-Type": "application/json",
      },
    },
  );
  const resp = await response.json();

  return resp.data.transactions;
}

async function loadInteractions(contractId, height, after) {
  let variables = {
    tags: [
      {
        name: "App-Name",
        values: ["SmartWeaveAction"],
      },
      {
        name: "Contract",
        values: [contractId],
      },
    ],
    blockFilter: {
      max: height,
    },
    first: MAX_REQUEST,
  };

  if (after !== undefined) {
    variables.after = after;
  }

  const tx = await nextPage(variables);
  const txs = tx.edges;

  while (tx.pageInfo.hasNextPage) {
    if(!txs[MAX_REQUEST - 1]) {
      return txs;
    }
    variables.after = txs[MAX_REQUEST - 1].cursor;
    const next = await nextPage(variables);

    txs.push(next.edges);
  }

  return txs;
}

me.addEventListener("message", (event) => {
  const { tx, height, last } = event.data;
  if (!last) {
    loadInteractions(tx, height).then(interactions => me.postMessage(interactions));
  } else {    
    loadInteractions(tx, height, last).then(interactions => me.postMessage(interactions));
  }
});
`;

const sources = [getTagSource, loadInteractionsSource];
const loadInteractionsBlob = isNode ? sources.join("\n") : new Blob(sources, {
  type: "application/javascript",
});
const loadInteractionsWorker = new Worker(
  isNode ? loadInteractionsBlob : URL.createObjectURL(loadInteractionsBlob),
  { eval: true, type: "module" },
);

export async function loadContract(tx) {
  loadContractWorker.postMessage({ tx });
  return new Promise((resolve, reject) => {
    // For Node.js
    isNode && loadContractWorker.once("message", (event) => {
      resolve(event);
      loadContractWorker.once("message", () => {});
    });
    loadContractWorker.onmessage = (event) => {
      resolve(event.data);
    };
    loadContractWorker.onerror = (error) => {
      reject(error);
    };
  });
}

export async function loadInteractions(tx, height) {
  loadInteractionsWorker.postMessage({ tx, height, last: false });
  return new Promise((resolve, reject) => {
    // For Node.js
    isNode && loadInteractionsWorker.once("message", (event) => {
      resolve(event);
      loadInteractionsWorker.once("message", (event) => {});
    });
    loadInteractionsWorker.onmessage = (event) => {
      resolve(event.data);
    };
    loadInteractionsWorker.onerror = (error) => {
      reject(error);
    };
  });
}

export async function updateInteractions(tx, height, last) {
  loadInteractionsWorker.postMessage({ tx, height, last });
  return new Promise((resolve, reject) => {
    // For Node.js
    isNode && loadInteractionsWorker.once("message", (event) => {
      resolve(event);
      loadInteractionsWorker.once("message", (event) => {});
    });
    loadInteractionsWorker.onmessage = (event) => {
      resolve(event.data);
    };
    loadInteractionsWorker.onerror = (error) => {
      reject(error);
    };
  });
}

export async function executeContract(
  contractId,
  height,
) {
  const cachedContract = localStorage.getItem(contractId);
  const cachedInteractions = localStorage.getItem(`${contractId}-interactions`);

  let [contract, interactions] = await Promise.all([
    cachedContract ? JSON.parse(cachedContract) : loadContract(contractId),
    cachedInteractions
      ? JSON.parse(cachedInteractions)
      : loadInteractions(contractId, height),
  ]);
  let updatePromise = [];
  if (cachedInteractions) {
    // So now we have the cached interactions
    // but we still need to ensure that the cached interactions are up to date.
    const lastEdge = interactions[interactions.length - 1];
    if (lastEdge) {
      updatePromise = updateInteractions(contractId, height, lastEdge.cursor);
    }
  }

  if (!cachedContract) {
    localStorage.setItem(contractId, JSON.stringify(contract));
  }
  if (!cachedInteractions) {
    localStorage.setItem(
      `${contractId}-interactions`,
      JSON.stringify(interactions),
    );
  }

  const { source, state, type } = contract;

  switch (type) {
    case "application/javascript":
      const rt = new Runtime(source, state, {});

      // Slower than `rt.executeInteractions` but more readable
      // 100 interactions in ~30.06ms.
      //
      // for (const interaction of interactions) {
      //    const input = interaction.node.tags.find(data => data.name === "Input");
      //    await rt.execute({ input, caller: interaction.node.owner.address });
      // }

      // Faster. At 100 interactions in about 3.68ms.
      await rt.executeInteractions(interactions);

      const updatedInteractions = await updatePromise;
      if (updatedInteractions.length > 0) {
        await rt.executeInteractions(updatedInteractions);
      }

      rt.destroy();

      return rt.state;
    case "application/wasm":
      const module = str2u8(source);
      const wasm = new WasmRuntime();
      await wasm.compile(
        module,
        {},
      );

      let currState = encode(state);
      for (const interaction of interactions) {
        const input = interaction.node.tags.find((data) =>
          data.name === "Input"
        );
        currState = wasm.call(
          currState,
          encode({
            input,
            caller: interaction.node.owner.address,
          }),
        );
      }

      return currState;
    case "application/octet-stream":
      // TODO(perf): Streaming initalization
      const res = await fetch(
        "https://github.com/three-em/3em/raw/js_library/js/evm/evm.wasm",
      );
      const evmModule = new Uint8Array(await res.arrayBuffer());
      const bytecode = hex(source);
      const _storage = hex(state);
      for (const interaction of interactions) {
        const input = interaction.node.tags.find((data) =>
          data.name === "Input"
        );

        const machine = new Machine(evmModule, hex(input));
        machine.execute(bytecode);
        result = machine.result;
      }

      return result;
    default:
      throw new Error(`Unsupported contract type: ${type}`);
  }
}

const encode = (s) => new TextEncoder().encode(JSON.stringify(s));
function str2u8(str) {
  const bufView = new Uint8Array(str.length);
  for (let i = 0; i < str.length; i++) {
    bufView[i] = str.charCodeAt(i);
  }
  return bufView;
}
