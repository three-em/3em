import { Runtime } from "../../js/web/index.js";

const baseUrl = "https://arweave.net";

function getTag(tx, field) {
  const encodedName = btoa(field);
  return atob(tx.tags.find((data) => data.name === encodedName)?.value || "");
}

export async function loadContract(contractId) {
  const response = await fetch(new URL(`/tx/${contractId}`, baseUrl).href);
  const tx = await response.json();

  const contractSrcTxID = getTag(tx, "Contract-Src");

  const contractSrcResponse = await fetch(
    new URL(`/tx/${contractSrcTxID}/data`, baseUrl).href,
  );
  const contractSrc = await contractSrcResponse.text();

  const source = atob(contractSrc.replace(/_/g, "/").replace(/-/g, "+"));
  let state = getTag(tx, "Init-State");

  if (!state) {
    const stateTx = getTag(tx, "Init-State-TX");
    if (stateTx) {
      const stateTxResponse = await fetch(
        new URL(`/tx/${stateTx}/data`, baseUrl).href,
      );
      const stateTxData = await stateTxResponse.text();
      state = atob(stateTxData.data.replace(/_/g, "/").replace(/-/g, "+"));
    } else {
      const txDataResonse = await fetch(
        new URL(`/tx/${contractId}/data`, baseUrl).href,
      );
      const txData = await txDataResonse.text();

      state = atob(txData.replace(/_/g, "/").replace(/-/g, "+"));
    }
  }

  const sourceTxResponse = await fetch(
    new URL(`/tx/${contractSrcTxID}`, baseUrl).href,
  );
  const sourceTx = await sourceTxResponse.json();
  const type = getTag(sourceTx, "Content-Type");

  return {
    source,
    state,
    type,
  };
}

const query =
  `query Transactions($tags: [TagFilter!]!, $blockFilter: BlockFilter!, $first: Int!, $after: String) {
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
}`;

const MAX_REQUEST = 100;

async function nextPage(variables) {
  const response = await fetch(
    new URL(`/graphql`, baseUrl).href,
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
  const { data } = await response.json();

  return data.transactions;
}

export async function loadInteractions(contractId, height) {
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

  const tx = await nextPage(variables);
  const txs = tx.edges;

  while (tx.pageInfo.hasNextPage) {
    variables.after = txs.edges[MAX_REQUEST - 1].cursor;
    const next = await nextPage(variables);

    txs.push(next.edges);
  }

  return txs;
}

export async function executeContract(
  contractId,
  height,
  cache,
) {
  const [contract, interactions] = await Promise.all([
    loadContract(contractId),
    loadInteractions(contractId, height),
  ]);

  const { source, state, type } = contract;

  switch (type) {
    case "application/javascript":
      const rt = new Runtime(source, state, {});
      for (const interaction of interactions) {
        const input = interaction.node.tags.find(data => data.name === "Input");
        await rt.execute({ input, caller: interaction.node.owner.address });
      }
      rt.destroy();
      return rt.state;
      break;
    case "application/wasm":
      break;
    case "application/octet-stream":
      break;
    default:
      throw new Error(`Unsupported contract type: ${type}`);
  }
}
