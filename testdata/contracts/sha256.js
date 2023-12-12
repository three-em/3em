export async function handle(state, action) {
    try {
      const someSha = await SmartWeave.convertString.toSHA256("hello");
      return { state: [someSha]  };
    } catch(e) {
      return { state: e.toString() }
    }
}
