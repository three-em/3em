// https://github.com/ArweaveTeam/SmartWeave/blob/master/examples/test-api-things.js
export async function handle(state, _action) {
  const txId = SmartWeave.transaction.id;
  const txOwner = SmartWeave.transaction.owner;
  const txTarget = SmartWeave.transaction.target;
  const txQuantity = SmartWeave.transaction.quantity;
  const txReward = SmartWeave.transaction.reward;
  const txTags = SmartWeave.transaction.tags;
  const blockHeight = SmartWeave.block.height;
  const blockIndepHash = SmartWeave.block.indep_hash;

  const ownerBytes = SmartWeave.arweave.utils.b64UrlToBuffer(txOwner);
  const from = SmartWeave.arweave.utils.bufferToB64Url(
    await SmartWeave.arweave.utils.crypto.hash(ownerBytes),
  );

  state.log = [...state.log, {
    blockHeight,
    blockIndepHash,
    txId,
    txOwner: txOwner,
    txTarget,
    txQuantity,
    txReward,
    txTags,
    from,
  }];
  return { state };
}
