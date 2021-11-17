use serde::Deserialize;
use serde::Serialize;

#[derive(Serialize, Deserialize)]
pub struct State {
  counter: i32,
}

#[derive(Deserialize)]
pub struct Action {}

fn neat_handle(state: State, _action: Action) -> State {
  State { counter: state.counter + 1 }
}

#[no_mangle]
pub extern fn handle(
  state: *const u8,
  state_size: usize,
  action: *const u8,
  action_size: usize,
  output: *mut *const u8,
  output_size: *mut usize,
) {
  let state_buf = unsafe { std::slice::from_raw_parts(state, state_size) };
  let state: State = serde_json::from_slice(state_buf).unwrap();
  let action_buf = unsafe { std::slice::from_raw_parts(action, action_size) };
  let action: Action = serde_json::from_slice(action_buf).unwrap();

  let output_state = neat_handle(state, action);
  let output_buf = serde_json::to_vec(&output_state).unwrap();
  unsafe {
    *output = output_buf.as_slice().as_ptr();
    *output_size = output_buf.len();
  }
}