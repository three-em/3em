import { Runtime } from "./sw.js";
import { hex, Machine } from "./evm/index.js";
import { WasmRuntime } from "./wasm.js";

const getTagSource = `
function getTag(tx, field) {
  const encodedName = btoa(field);
  return atob(tx.tags.find((data) => data.name === encodedName)?.value || "");
}`;

const loadContractSource = `
const baseUrl = "https://arweave.net";
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

self.addEventListener("message", async (event) => {
  const { tx, key } = event.data;
  const r = await loadContract(tx);
  self.postMessage({ key, result: r });
});
`;

const loadContractSources = [getTagSource, loadContractSource];
const loadContractBlob = new Blob(loadContractSources, {
    type: "application/javascript",
  });
const loadContractWorker = new Worker(
  URL.createObjectURL(loadContractBlob),
  { eval: true, type: "module" },
);

const loadInteractionsSource = `
const baseUrl = "https://arweave.net";
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

const MAX_REQUEST = 100;

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

  let tx = await nextPage(variables);
  const txs = tx.edges;
  let lastOfMax = txs[MAX_REQUEST - 1];
  
  let getLastTxInArray = () => txs[txs.length - 1];
  
  while (tx.edges.length > 0) {
  
    if(!lastOfMax) {
      return txs;
    }
    
    variables.after = getLastTxInArray().cursor;
    tx = await nextPage(variables);
    txs.push(...tx.edges);
  }
  return txs;
}

self.addEventListener("message", async (event) => {
  const { tx, height, last, key } = event.data;
  let interactions;
  if (!last) {
    interactions = await loadInteractions(tx, height)
  } else {    
    interactions = await loadInteractions(tx, height, last);
  }
  self.postMessage({ key, result: interactions });
});
`;

const sources = [getTagSource, loadInteractionsSource];
const loadInteractionsBlob = new Blob(sources, {
  type: "application/javascript",
});
const loadInteractionsSourceURL = isNode
  ? loadInteractionsBlob
  : URL.createObjectURL(loadInteractionsBlob);
let loadInteractionsWorker = new Worker(
  loadInteractionsSourceURL,
  { eval: true, type: "module" },
);

const contractProcessingQueue = {};
let k = 0;
loadContractWorker.onmessage = (event) => {
  const p = contractProcessingQueue[event.data.key];
  p(event.data.result);
};
loadContractWorker.onerror = (error) => {
  throw error;
};

export async function loadContract(tx) {
  const key = k++;
  const args = { tx, key };
  return new Promise((r) => {
    loadContractWorker.postMessage(args);
    contractProcessingQueue[key] = r;
  });
}

const interactionProcessingQueue = {};

loadInteractionsWorker.onmessage = (event) => {
  const p = interactionProcessingQueue[event.data.key];
  p(event.data.result);
};
loadInteractionsWorker.onerror = (error) => {
  throw error;
};

export function loadInteractions(tx, height) {
  return updateInteractions(tx, height, false);
}

export function updateInteractions(tx, height, last) {
  const key = k++;
  const args = { tx, height, last, key };

  return new Promise((r) => {
    loadInteractionsWorker.postMessage(args);
    interactionProcessingQueue[key] = r;
  });
}

export async function executeContract(
  contractId,
  height,
  clearCache,
) {
  if (clearCache) {
    localStorage.clear();
  }
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
