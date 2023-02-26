export async function handle(state, action) {
    state.counts++;
    let stateObj = { state };
    if(state.counts > 1) {
        throw new Error("An error has been thrown");
    }
    return stateObj;
}
