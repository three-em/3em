export async function handle(state, action) {
    try {
        const x = await SmartWeave.contracts.readContractState("0zplqhFARjHyR-dBdNEY1TpZuKL0mWm--RFq6LByoew", 921589, false);
        return {
            state: x
        }
    } catch (e) {
        return {
            state: e.toString()
        }
    }
}
