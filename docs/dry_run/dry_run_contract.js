export async function handle(state, action) {
 if(action.input.function === 'add') {
     state.users.push(action.input.name);
 } else {
     throw new Error("Invalid operation");
 }

 return {
     state
 };
}
