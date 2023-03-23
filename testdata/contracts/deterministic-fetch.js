export async function handle(state, action) {
    const ethTxId = action.input.id;
    const fetchTx = await EXM.deterministicFetch(`https://api.blockcypher.com/v1/eth/main/txs/${ethTxId}`);
    const txJson = fetchTx.asJSON();
    state[ethTxId] = txJson.fees;
    return {
        state,
        result: txJson
    }
}