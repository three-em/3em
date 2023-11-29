export async function handle(state, action) {
    try {
      const someSha = await EXM.stringToSHA256("hello");
      return { state: [someSha]  };
    } catch(e) {
      return { state: e.toString() }
    }
}
