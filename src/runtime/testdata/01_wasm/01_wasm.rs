use serde::Deserialize;
use serde::Serialize;
use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::Layout;

// #[link(wasm_import_module = "3em")]
// extern {
//   fn Smartweave_ReadState(*mut u8) -> usize;
// }

#[derive(Deserialize)]
pub struct Tag {
  pub name: String,
  pub value: String,
}

#[derive(Deserialize)]
pub struct ContractTx {
  pub id: String,
  pub owner: String,
  pub tags: Vec<Tag>,
  pub target: String,
  pub quantity: String,
  pub reward: String,
}

#[derive(Deserialize)]
pub struct ContractBlock {
  pub height: usize,
  pub indep_hash: String,
  pub timestamp: String,
}

#[derive(Deserialize)]
pub struct ContractInfo {
  pub transaction: ContractTx,
  pub block: ContractBlock,
}

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

fn neat_handle(state: State, _action: Action, contract_info: ContractInfo) -> State {
  assert_eq!(contract_info.transaction.id, "");
  assert_eq!(contract_info.transaction.owner, "");
  assert_eq!(contract_info.transaction.tags.len(), 0);
  assert_eq!(contract_info.transaction.target, "");
  assert_eq!(contract_info.transaction.quantity, "");
  assert_eq!(contract_info.transaction.reward, "");
  assert_eq!(contract_info.block.height, 0);
  assert_eq!(contract_info.block.indep_hash, "");
  assert_eq!(contract_info.block.timestamp, "");
  
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
  contract_info_ptr: *mut u8,
  contract_info_size: usize,
) -> *const u8 {
  let state_buf = unsafe { Vec::from_raw_parts(state, state_size, state_size) };
  let state: State = serde_json::from_slice(&state_buf).unwrap();

  let action_buf =
    unsafe { Vec::from_raw_parts(action, action_size, action_size) };
  let action: Action = serde_json::from_slice(&action_buf).unwrap();

  let contract_info_buf =
    unsafe { Vec::from_raw_parts(contract_info_ptr, contract_info_size, contract_info_size) };
  let contract_info: ContractInfo = serde_json::from_slice(&contract_info_buf).unwrap();

  let output_state = neat_handle(state, action, contract_info);
  let output_buf = serde_json::to_vec(&output_state).unwrap();
  let output = output_buf.as_slice().as_ptr();

  unsafe {
    LEN = output_buf.len();
  }

  std::mem::forget(state_buf);
  std::mem::forget(contract_info_buf);
 
  output
}
