/**
 *
 * @param state is the current state your application holds
 * @param action is an object containing { input, caller } . Most of the times you will only use `action.input` which contains the input passed as a write operation
 * @returns {Promise<{ users: Array<{ username: string}> }>}
 */
export async function handle(state, action) {
    const { username } = action.input;
    state.users.push({ username });
    return state;
}