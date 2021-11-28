use three_em::handler;
use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct State {
  counter: i32,
}

#[derive(Deserialize)]
pub struct Action {}

#[handler]
pub fn handle(state: State, _action: Action) -> State {
  State {
    counter: state.counter + 1,
  }
}
