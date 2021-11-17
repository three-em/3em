use serde::Deserialize;
use serde::Serialize;
use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::Layout;

#[no_mangle]
pub unsafe fn _alloc(len: usize) -> *mut u8 {
  let align = std::mem::align_of::<usize>();
  let layout = Layout::from_size_align_unchecked(len, align);
  alloc(layout)
}

#[no_mangle]
pub unsafe fn _dealloc(ptr: *mut u8, size: usize) {
  let align = std::mem::align_of::<usize>();
  let layout = Layout::from_size_align_unchecked(size, align);
  dealloc(ptr, layout);
}

#[derive(Serialize, Deserialize, Default)]
pub struct State {
  counter: i32,
}

#[derive(Deserialize)]
pub struct Action {}

fn neat_handle(state: State, _action: Action) -> State {
  State {
    counter: state.counter + 1,
  }
}

static mut LEN: usize = 0;

#[no_mangle]
pub extern "C" fn get_len() -> usize {
  unsafe { LEN }
}

#[no_mangle]
pub extern "C" fn handle(
  state: *mut u8,
  state_size: usize,
  action: *mut u8,
  action_size: usize,
) -> *const u8 {
  let state_buf = unsafe { Vec::from_raw_parts(state, state_size, state_size) };
  let state: State = serde_json::from_slice(&state_buf).unwrap();

  let action_buf =
    unsafe { Vec::from_raw_parts(action, action_size, action_size) };
  let action: Action = serde_json::from_slice(&action_buf).unwrap();

  let output_state = neat_handle(state, action);
  let output_buf = serde_json::to_vec(&output_state).unwrap();
  let output = output_buf.as_slice().as_ptr();

  unsafe {
    LEN = output_buf.len();
  }

  std::mem::forget(state_buf);
  output
}
