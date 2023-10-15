export async function handle(state, action) {
    const input = action.input;
    const { gte, lt, reverse, limit } = input;
    // feed these inputs into a kvmap
    const smwResult = await SmartWeave.kv.keys(gte, lt, reverse, limit);
    // take in the arguments and test each functionality out one by one. 
    return { result: JSON.stringify(smwResult) }
}
