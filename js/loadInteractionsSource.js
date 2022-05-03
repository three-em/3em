export const loadInteractionsSource = (arweaveUrl) => `
const baseUrl = "${arweaveUrl}";
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

async function nextPage(variables, baseUrlCustom) {
  const response = await fetch(
    new URL("/graphql", baseUrlCustom || baseUrl).href,
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

async function loadInteractions(contractId, height, after, baseUrlCustom) {
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

  let tx = await nextPage(variables, baseUrlCustom);
  const txs = tx.edges;
  let lastOfMax = txs[MAX_REQUEST - 1];
  
  let getLastTxInArray = () => txs[txs.length - 1];
  
  while (tx.edges.length > 0) {
  
    if(!lastOfMax) {
      return txs;
    }
    
    variables.after = getLastTxInArray().cursor;
    tx = await nextPage(variables, baseUrlCustom);
    txs.push(...tx.edges);
  }
  return txs;
}

self.addEventListener("message", async (event) => {
  const { tx, height, last, key, baseUrlCustom } = event.data;
  let interactions;
  if (!last) {
    interactions = await loadInteractions(tx, height, undefined, baseUrlCustom)
  } else {    
    interactions = await loadInteractions(tx, height, last, baseUrlCustom);
  }
  self.postMessage({ key, result: interactions });
});
`;
