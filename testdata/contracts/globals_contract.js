export function handle(state, action) {
    const data = Deno.core.opSync("op_smartweave_init", new Uint8Array());
    return {
        state: {
            ...data,
            // ...state,
            // txId: SmartWeave.transaction.id,
            // txOwner: SmartWeave.transaction.owner,
            // txTarget: SmartWeave.transaction.target,
            // txQuantity: SmartWeave.transaction.quantity,
            // txReward: SmartWeave.transaction.reward,
            // txTags: SmartWeave.transaction.tags,
            // txHeight: SmartWeave.block.height,
            // txIndepHash: SmartWeave.block.indep_hash,
            // txTimestamp: SmartWeave.block.timestamp,
            // winstonToAr: SmartWeave.arweave.ar.winstonToAr(1) === "0.000000000001",
            // arToWinston: SmartWeave.arweave.ar.arToWinston(1) === "1000000000000",
            // compareArWinston: SmartWeave.arweave.ar.compare(1) === "1000000000000"
        }
    }
}
