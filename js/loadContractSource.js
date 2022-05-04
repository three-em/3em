export const loadContractSource = (arweaveUrl) => `
const baseUrl = "${arweaveUrl}";
async function loadContract(contractId, baseUrlCustom) {
  const response = await fetch(new URL(\`/tx/\${contractId}\`, baseUrlCustom || baseUrl).href);
  const tx = await response.json();

  const contractSrcTxID = getTag(tx, "Contract-Src");

  const contractSrcResponse = await fetch(
    new URL(\`/tx/\${contractSrcTxID}/data\`, baseUrlCustom || baseUrl).href,
  );
  const contractSrc = await contractSrcResponse.text();
  
  const source = atob(contractSrc.replace(/_/g, "/").replace(/-/g, "+"));
  let state = getTag(tx, "Init-State");

  if (!state) {
    const stateTx = getTag(tx, "Init-State-TX");
    if (stateTx) {
      const stateTxResponse = await fetch(
        new URL(\`/tx/\${stateTx}/data\`, baseUrlCustom || baseUrl).href,
      );
      const stateTxData = await stateTxResponse.text();
      state = atob(stateTxData.data.replace(/_/g, "/").replace(/-/g, "+"));
    } else {
      const txDataResonse = await fetch(
        new URL(\`/tx/\${contractId}/data\`, baseUrlCustom || baseUrl).href,
      );
      const txData = await txDataResonse.text();

      state = atob(txData.replace(/_/g, "/").replace(/-/g, "+"));
    }
  }
  const sourceTxResponse = await fetch(
    new URL(\`/tx/\${contractSrcTxID}\`, baseUrlCustom || baseUrl).href,
  );
  const sourceTx = await sourceTxResponse.json();
  const type = getTag(sourceTx, "Content-Type");

  return {
    source,
    state,
    type,
    tx
  };
}

self.addEventListener("message", async (event) => {
  const { tx, key, baseUrlCustom } = event.data;
  const r = await loadContract(tx, baseUrlCustom);
  self.postMessage({ key, result: r });
});
`;
