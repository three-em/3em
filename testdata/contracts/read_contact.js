export async function handle(state, action) {
    try {
        return {
            state: await SmartWeave.contracts.readContractState("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE", 921589, false)
        }
    } catch (e) {
        return {
            state: e.toString()
        }
    }
}
