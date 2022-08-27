export async function addUser(state, action) {
    state.users.push(action.input.username);
    return state;
}

export async function handle(state, action) {

    if(action.input.function === 'addUser') {
        state.users.push(action.input.username);
    } else {
        throw new Error("Invalid operation");
    }

    return {
        state
    };
}
