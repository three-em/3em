use deno_core::error::AnyError;
use deno_core::include_js_files;
use deno_ops::op;

use deno_core::serde::{Deserialize, Serialize};
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

pub struct ExecutorSettings {
  settings: HashMap<String, Value>,
}

#[derive(Deserialize)]
pub struct DeterministicFetchOptions {
  url: String,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct DeterministicFetchBody {
  #[serde(rename = "type")]
  pub req_type: String,
  pub url: String,
  pub statusText: String,
  pub status: i8,
  pub redirected: bool,
  pub ok: bool,
  pub headers: HashMap<String, String>,
  pub vector: Vec<u8>,
}

#[derive(Deserialize, Serialize, Default, Clone)]
pub struct ExmContext {
  pub requests: HashMap<String, DeterministicFetchBody>,
  pub kv: HashMap<String, deno_core::serde_json::Value>,
  pub initiated: Vec<String>,
}

pub fn init(executor_settings: HashMap<String, Value>) -> Extension {
  Extension::builder()
    .js(include_js_files!(
      prefix "3em:baseops",
      "base.js",
    ))
    .ops(vec![
      op_get_executor_settings::decl(),
      op_exm_write_to_console::decl(),
    ])
    .state(move |state| {
      state.put(ExecutorSettings {
        settings: executor_settings.clone(),
      });
      Ok(())
    })
    .build()
}

#[op]
pub fn op_get_executor_settings(
  _state: &mut OpState,
  setting: String,
  _: (),
) -> Result<Value, AnyError> {
  let s = _state;
  let settings = s.borrow::<ExecutorSettings>();
  if let Some(data) = settings.settings.get(&setting) {
    Ok(data.clone())
  } else {
    Ok(Value::Null)
  }
}

#[op]
pub fn op_exm_write_to_console(_: &mut OpState, content: String, _: ()) {
  println!("{}", content);
}
