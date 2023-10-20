export async function handle(state, action) {
    const input = action.input;

    if (input.function === "put") {
      const { key, value } = input;
      state.puts.push({ key, value }); // write to state
      SmartWeave.kv.put(key, value); // write to KVS
      return { result: JSON.stringify(SmartWeave.kv.getAll()) }; // state
    }
    if (input.function === "get") {
      const { key } = input;

      const res = SmartWeave.kv.get(key);
      state.gets.push(res)
      return { state };
    }
  
    if (input.function === "del") {
      const { key } = input;
      SmartWeave.kv.del(key);
      return { state };
    }
  }

