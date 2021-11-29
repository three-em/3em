use deno_core::error::type_error;
use deno_core::error::AnyError;
use deno_core::include_js_files;
use deno_core::op_async;
use deno_core::op_sync;
use deno_core::serde::Serialize;
use deno_core::serde_json::Value;
use deno_core::Extension;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

pub fn init() -> Extension {
  Extension::builder()
    .js(include_js_files!(
      prefix "3em:smartweave",
      "src/runtime/bignumber.js",
      "src/runtime/smartweave.js",
      "src/runtime/contract-assert.js",
    ))
    .ops(vec![
      ("op_smartweave_init", op_sync(op_smartweave_init)),
      (
        "op_smartweave_wallet_balance",
        op_async(op_smartweave_wallet_balance),
      ),
      (
        "op_smartweave_wallet_last_tx",
        op_async(op_smartweave_wallet_last_tx),
      ),
    ])
    .build()
}

#[derive(Serialize, Default)]
pub struct Tag {
  pub name: String,
  pub value: String,
}

#[derive(Serialize, Default)]
pub struct ContractTx {
  pub id: String,
  pub owner: String,
  pub tags: Vec<Tag>,
  pub target: String,
  pub quantity: String,
  pub reward: String,
}

#[derive(Serialize, Default)]
pub struct ContractBlock {
  pub height: usize,
  pub indep_hash: String,
  pub timestamp: String,
}

#[derive(Serialize, Default)]
pub struct ContractInfo {
  pub transaction: ContractTx,
  pub block: ContractBlock,
}

pub fn op_smartweave_init(
  state: &mut OpState,
  _zero_copy: ZeroCopyBuf,
  _: (),
) -> Result<ContractInfo, AnyError> {
  let contract = state
    .try_take::<ContractInfo>()
    .ok_or_else(|| type_error("Contract info missing."))?;

  Ok(contract)
}

pub async fn op_smartweave_wallet_balance(
  _state: Rc<RefCell<OpState>>,
  address: String,
  _: (),
) -> Result<String, AnyError> {
  // Winston string
  let balance =
    reqwest::get(format!("https://arweave.net/wallet/{}/balance", address))
      .await?
      .text()
      .await?;
  Ok(balance)
}

pub async fn op_smartweave_wallet_last_tx(
  _state: Rc<RefCell<OpState>>,
  address: String,
  _: (),
) -> Result<String, AnyError> {
  let tx =
    reqwest::get(format!("https://arweave.net/wallet/{}/last_tx", address))
      .await?
      .text()
      .await?;
  Ok(tx)
}

pub fn read_contract_state(id: String) -> Value {
  // We want this to be a synchronous operation
  // because of its use with v8::Function.
  // But, Tokio will panic if we make blocking calls,
  // so we need offload it to a thread.
  thread::spawn(move || {
    println!("Reading contract state for {}", id);
    let state: Value = reqwest::blocking::get(format!(
      "https://storage.googleapis.com/verto-exchange-contracts/{}/{}_state.json",
      id, id,
    ))
    .unwrap()
    .json()
    .unwrap();
    state
  })
  .join()
  .unwrap()
}
