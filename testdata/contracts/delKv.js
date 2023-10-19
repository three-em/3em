
export async function handle(state, action) {
    const input = action.input;
    const { key, value } = input;
    await SmartWeave.kv.del(key);
    return { result: JSON.stringify(SmartWeave.kv.getAll()) }
}