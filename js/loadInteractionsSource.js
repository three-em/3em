export const queryGraphql = `query Transactions($tags: [TagFilter!]!, $blockFilter: BlockFilter!, $first: Int!, $after: String) {
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

async function nextPage(query, variables, gatewayUrl) {
    const response = await fetch(
        new URL("/graphql", gatewayUrl).href,
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

async function loadInteractions(opts) {
    const { MAX_REQUEST, contractId, height, after, gatewayUrl, query } = opts;
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

    let tx = await nextPage(query, variables, gatewayUrl);
    let txs = tx.edges;
    let lastOfMax = txs[MAX_REQUEST - 1];

    let getLastTxInArray = () => txs[txs.length - 1];

    while (tx.edges.length > 0) {

        if(!lastOfMax) {
            return txs;
        }

        variables.after = getLastTxInArray().cursor;
        tx = await nextPage(query, variables, gatewayUrl);
        txs.push(...tx.edges);
    }

    txs = txs.filter(
        (tx) => !tx.node.bundledIn || !tx.node.bundledIn?.id || !tx.node.parent || !tx.node.parent?.id,
    )

    return txs;
}

const processEvent = (query, maxRequests, baseUrl) => async (event) => {
    const { tx, height, last, key, baseUrlCustom } = event.data;
    const gatewayUrl = baseUrlCustom || baseUrl;
    let interactions;
    if (!last) {
        interactions = await loadInteractions({
            contractId: tx,
            height,
            after: undefined,
            gatewayUrl,
            query,
            MAX_REQUEST: maxRequests
        })
    } else {
        interactions = await loadInteractions({
            contractId: tx,
            height: height,
            after: last,
            gatewayUrl,
            query,
            MAX_REQUEST: maxRequests
        });
    }
    self.postMessage({ key, result: interactions });
}

export const loadInteractionsSource = (arweaveUrl) => `
const baseUrl = "${arweaveUrl}";
const query = \`${queryGraphql}\`;

const MAX_REQUEST = 100;

${nextPage.toString()}
${loadInteractions.toString()}

self.addEventListener("message", (${processEvent.toString()})(query, MAX_REQUEST, baseUrl));
`;
