function getTag(tx, field) {
    const encodedName = btoa(field);
    return atob(tx.tags.find((data) => data.name === encodedName)?.value || "");
}

async function loadContract(contractId, gatewayUrl, contractSrcTxId) {
    const response = await fetch(new URL(`/tx/${contractId}`, gatewayUrl).href);
    const tx = await response.json();

    const contractSrcTxID = contractSrcTxId || getTag(tx, "Contract-Src");

    const contractSrcResponse = await fetch(
        new URL(`/tx/${contractSrcTxID}/data`, gatewayUrl).href,
    );
    const contractSrc = await contractSrcResponse.text();

    const source = atob(contractSrc.replace(/_/g, "/").replace(/-/g, "+"));
    let state = getTag(tx, "Init-State");

    if (!state) {
        const stateTx = getTag(tx, "Init-State-TX");
        if (stateTx) {
            const stateTxResponse = await fetch(
                new URL(`/tx/${stateTx}/data`, gatewayUrl).href,
            );
            const stateTxData = await stateTxResponse.text();
            state = atob(stateTxData.data.replace(/_/g, "/").replace(/-/g, "+"));
        } else {
            const txDataResonse = await fetch(
                new URL(`/tx/${contractId}/data`, gatewayUrl).href,
            );
            const txData = await txDataResonse.text();

            state = atob(txData.replace(/_/g, "/").replace(/-/g, "+"));
        }
    }
    const sourceTxResponse = await fetch(
        new URL(`/tx/${contractSrcTxID}`, gatewayUrl).href,
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

const processEvent = (baseUrl) => async (event) => {
    const { tx, key, baseUrlCustom, contractSrcTxId } = event.data;
    const r = await loadContract(tx, baseUrlCustom || baseUrl, contractSrcTxId);
    self.postMessage({ key, result: r });
};

export const loadContractSource = (arweaveUrl) => `
const baseUrl = "${arweaveUrl}";

${getTag.toString()}
${loadContract.toString()}

self.addEventListener("message", (${processEvent.toString()})(baseUrl));
`;
