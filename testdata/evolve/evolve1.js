export function handle(state, action) {
  switch (action.input.function) {
    case 'evolve':
      return {
        state: {
          ...state,
          canEvolve: true,
          evolve: action.input.value,
        }
      };
    default:
      throw new ContractError('Invalid function');
  }
}
