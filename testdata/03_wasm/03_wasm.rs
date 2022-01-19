use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::alloc::alloc;
use std::alloc::dealloc;
use std::alloc::Layout;

#[link(wasm_import_module = "3em")]
extern "C" {
  fn smartweave_read_state(
    // `ptr` is the pointer to the base64 URL encoded sha256 txid.
    ptr: *const u8,
    ptr_len: usize,
    // Pointer to the 4 byte array to store the length of the state.
    result_len_ptr: *mut u8,
  ) -> *mut u8;
}

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
  pub timestamp: usize,
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
  txId: String,
  owner: String,
  height: usize,
}

#[derive(Deserialize)]
pub struct Action {}

fn neat_read_state(tx_id: &[u8]) -> Value {
  let mut len = [0u8; 4];
  let state_ptr = unsafe {
    smartweave_read_state(tx_id.as_ptr(), tx_id.len(), len.as_mut_ptr())
  };

  let len = u32::from_le_bytes(len) as usize;
  let state = unsafe { Vec::from_raw_parts(state_ptr, len, len) };

  serde_json::from_slice(&state).unwrap()
}

fn neat_handle(
  state: State,
  _action: Action,
  contract_info: ContractInfo,
) -> State {
  State {
    txId: contract_info.transaction.id,
    owner: contract_info.transaction.owner,
    height: contract_info.block.height,
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

  let contract_info_buf = unsafe {
    Vec::from_raw_parts(
      contract_info_ptr,
      contract_info_size,
      contract_info_size,
    )
  };
  let contract_info: ContractInfo =
    serde_json::from_slice(&contract_info_buf).unwrap();

  let output_state = neat_handle(state, action, contract_info);
  let output_buf = serde_json::to_vec(&output_state).unwrap();
  let output = output_buf.as_slice().as_ptr();

  unsafe {
    LEN = output_buf.len();
  }

  std::mem::forget(state_buf);
  std::mem::forget(action_buf);
  std::mem::forget(contract_info_buf);

  output
}
