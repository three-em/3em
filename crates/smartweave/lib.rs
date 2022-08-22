use deno_core::error::AnyError;
use deno_core::include_js_files;
use deno_core::OpDecl;
use deno_ops::op;

use deno_core::serde::Serialize;
use deno_core::serde_json::Value;
use deno_core::Extension;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;
use std::cell::RefCell;
use std::collections::HashMap;
use std::future::Future;
use std::rc::Rc;
use std::{env, thread};
use three_em_arweave::gql_result::GQLTagInterface;

pub struct ArweaveInfo {
  pub port: i32,
  pub host: String,
  pub protocol: String,
}

pub struct ExecutorSettings {
  settings: HashMap<String, deno_core::serde_json::Value>,
}

#[derive(Serialize, Default, Clone)]
pub struct InteractionTx {
  pub id: String,
  pub owner: String,
  pub tags: Vec<GQLTagInterface>,
  pub target: String,
  pub quantity: String,
  pub reward: String,
}

#[derive(Serialize, Default, Clone)]
pub struct InteractionBlock {
  pub height: usize,
  pub indep_hash: String,
  pub timestamp: usize,
}

#[derive(Serialize, Default, Clone)]
pub struct InteractionContext {
  pub transaction: InteractionTx,
  pub block: InteractionBlock,
  pub specific_function: Option<String>,
}

pub fn init(
  arweave: (i32, String, String),
  op_smartweave_read_contract: OpDecl,
) -> Extension {
  Extension::builder()
    .js(include_js_files!(
      prefix "3em:smartweave",
      "arweave.js",
      "bignumber.js",
      "smartweave.js",
      "contract-assert.js",
    ))
    .ops(vec![
      op_smartweave_unsafe_exit_process::decl(),
      op_smartweave_wallet_balance::decl(),
      op_smartweave_wallet_last_tx::decl(),
      op_smartweave_get_tx_data::decl(),
      op_smartweave_get_tx::decl(),
      op_smartweave_get_host::decl(),
      op_smartweave_read_contract,
    ])
    .state(move |state| {
      let (port, host, protocol) = arweave.clone();
      state.put(ArweaveInfo {
        port,
        host,
        protocol,
      });
      Ok(())
    })
    .build()
}

#[op]
pub fn op_smartweave_unsafe_exit_process(
  _state: &mut OpState,
  _: (),
  _: (),
) -> Result<(), AnyError> {
  println!("Unsafe calls have been invoked outside of a safe context");
  std::process::exit(1)
}

#[op]
pub async fn op_smartweave_wallet_balance(
  _state: Rc<RefCell<OpState>>,
  address: String,
  _: (),
) -> Result<String, AnyError> {
  let s = _state.borrow();

  let arweave = s.borrow::<ArweaveInfo>();

  // Winston string
  let balance =
    reqwest::get(format!("{}/wallet/{}/balance", get_host(arweave), address))
      .await?
      .text()
      .await?;
  Ok(balance)
}

#[op]
pub async fn op_smartweave_wallet_last_tx(
  _state: Rc<RefCell<OpState>>,
  address: String,
  _: (),
) -> Result<String, AnyError> {
  let s = _state.borrow();
  let arweave = s.borrow::<ArweaveInfo>();

  let tx =
    reqwest::get(format!("{}/wallet/{}/last_tx", get_host(arweave), address))
      .await?
      .text()
      .await?;
  Ok(tx)
}

#[op]
pub async fn op_smartweave_get_tx_data(
  _state: Rc<RefCell<OpState>>,
  tx_id: String,
  _: (),
) -> Result<ZeroCopyBuf, AnyError> {
  let s = _state.borrow();
  let arweave = s.borrow::<ArweaveInfo>();

  let req = reqwest::get(format!("{}/{}", get_host(arweave), tx_id))
    .await?
    .bytes()
    .await?;

  Ok(req.to_vec().into())
}

#[op]
pub async fn op_smartweave_get_tx(
  _state: Rc<RefCell<OpState>>,
  tx_id: String,
  _: (),
) -> Result<String, AnyError> {
  let s = _state.borrow();
  let arweave = s.borrow::<ArweaveInfo>();

  let req = reqwest::get(format!("{}/tx/{}", get_host(arweave), tx_id))
    .await?
    .text()
    .await?;

  Ok(req)
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

pub fn get_host(arweave_info: &ArweaveInfo) -> String {
  if arweave_info.port == 80 {
    format!("{}://{}", arweave_info.protocol, arweave_info.host)
  } else {
    format!(
      "{}://{}:{}",
      arweave_info.protocol, arweave_info.host, arweave_info.port
    )
  }
}

#[op]
pub async fn op_smartweave_get_host(
  _state: Rc<RefCell<OpState>>,
  _parameters: (),
  _: (),
) -> Result<String, AnyError> {
  let s = _state.borrow();
  let arweave = s.borrow::<ArweaveInfo>();
  Ok(get_host(arweave))
}
