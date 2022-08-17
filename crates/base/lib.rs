use deno_core::error::AnyError;
use deno_core::include_js_files;
use deno_core::op_async;
use deno_core::op_sync;

use deno_core::serde::{Serialize, Deserialize};
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

pub fn init(executor_settings: HashMap<String, Value>) -> Extension
{
  Extension::builder()
    .js(include_js_files!(
      prefix "3em:baseops",
      "base.js",
    ))
    .ops(vec![(
      "op_executor_settings",
      op_async(op_get_executor_settings),
    )])
    .state(move |state| {
      state.put(ExecutorSettings {
        settings: executor_settings.clone(),
      });
      Ok(())
    })
    .build()
}

pub async fn op_deterministic_fetch(
  _state: Rc<RefCell<OpState>>,
  options: DeterministicFetchOptions,
  _: (),
) -> Result<String, AnyError> {
Ok("".into())
}

pub async fn op_get_executor_settings(
  _state: Rc<RefCell<OpState>>,
  setting: String,
  _: (),
) -> Result<Value, AnyError> {
  let s = _state.borrow();
  let settings = s.borrow::<ExecutorSettings>();
  if let Some(data) = settings.settings.get(&setting) {
    Ok(data.clone())
  } else {
    Ok(Value::Null)
  }
}