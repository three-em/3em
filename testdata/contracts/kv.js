export async function handle(state, action) {
    const input = action.input;
    const { key, value } = input;
    SmartWeave.kv.put(key, value);
    return { result: JSON.stringify(SmartWeave.kv.getAll()) }
}