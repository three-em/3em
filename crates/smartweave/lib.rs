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
use three_em_arweave::arweave::TransactionData;

pub struct ArweaveInfo {
  port: i32,
  host: String,
  protocol: String,
}

pub fn init(info: ContractInfo, arweave: (i32, String, String)) -> Extension {
  Extension::builder()
    .js(include_js_files!(
      prefix "3em:smartweave",
      "bignumber.js",
      "smartweave.js",
      "contract-assert.js",
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
      ("op_smartweave_get_host", op_async(op_smartweave_get_host)),
    ])
    .state(move |state| {
      state.put(info.clone());
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

#[derive(Serialize, Default, Clone)]
pub struct ContractBlock {
  pub height: usize,
  pub indep_hash: String,
  pub timestamp: String,
}

#[derive(Serialize, Default, Clone)]
pub struct ContractInfo {
  pub transaction: TransactionData,
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

pub async fn op_smartweave_get_host(
  _state: Rc<RefCell<OpState>>,
  _parameters: (),
  _: (),
) -> Result<String, AnyError> {
  let s = _state.borrow();
  let arweave = s.borrow::<ArweaveInfo>();
  Ok(get_host(arweave))
}
