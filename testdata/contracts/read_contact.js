export async function handle(state, action) {
    try {
        return {
            state: await SmartWeave.contracts.readContractState("0zplqhFARjHyR-dBdNEY1TpZuKL0mWm--RFq6LByoew", 921589, false)
        }
    } catch (e) {
        return {
            state: e.toString()
        }
    }
}
