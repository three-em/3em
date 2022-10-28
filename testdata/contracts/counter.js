export async function handle(state, action) {
    state.counts++;
    let stateObj = { state };
    if(state.counts <= 1) {
        stateObj.result = 'Some result';
    }
    return stateObj;
}
