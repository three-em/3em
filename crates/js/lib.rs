pub mod default_permissions;
mod loader;
pub mod snapshot;

use crate::default_permissions::Permissions;
use crate::loader::EmbeddedModuleLoader;
use deno_core::error::{generic_error, AnyError};
use deno_core::serde::de::DeserializeOwned;
use deno_core::serde::Serialize;
use deno_core::serde_json::Value;
use deno_core::serde_v8;
use deno_core::JsRuntime;
use deno_core::OpDecl;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use deno_fetch::Options;
use deno_web::BlobStore;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::future::Future;
use std::rc::Rc;
use three_em_smartweave::InteractionContext;
use v8::HandleScope;

#[derive(Debug, Clone)]
pub enum HeapLimitState {
  /// Ok, the heap limit is not exceeded.
  Ok,
  /// The heap limit is exceeded.
  Exceeded(usize),
}

impl Default for HeapLimitState {
  fn default() -> Self {
    HeapLimitState::Ok
  }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
  /// Isolate is terminated.
  Terminated,
}

impl std::fmt::Display for Error {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Error::Terminated => write!(f, "Isolate is terminated"),
    }
  }
}

impl std::error::Error for Error {}

// TODO(@littledivy): Maybe add a Null variant?
#[derive(Debug, PartialEq)]
pub enum CallResult {
  // Contract wants to "evolve"
  Evolve(String),
  // Result, was state updated?
  Result(v8::Global<v8::Value>, bool),
}

unsafe impl Send for Runtime {}
unsafe impl Sync for Runtime {}

unsafe impl Send for EmbeddedModuleLoader {}
unsafe impl Sync for EmbeddedModuleLoader {}

pub struct Runtime {
  rt: JsRuntime,
  module: v8::Global<v8::Value>,
  pub state: Rc<RefCell<HeapLimitState>>,
  /// Optimization to avoid running the event loop in certain cases.
  ///
  /// None, if the handler is not yet called.
  /// Some(true), if the handler is called and it returns a pending promise.
  /// Some(false), if the handler is called and it does not return a pending promise.
  is_promise: Option<bool>,
  /// Current state value.
  contract_state: v8::Global<v8::Value>,

  /// Whether the current runtime belongs to EXM execution
  is_exm: bool,
}

impl Runtime {
  pub async fn new<T>(
    source: &str,
    init: T,
    arweave: (i32, String, String),
    op_smartweave_read_state: OpDecl,
    executor_settings: HashMap<String, deno_core::serde_json::Value>,
    maybe_exm_context: Option<deno_core::serde_json::Value>,
  ) -> Result<Self, AnyError>
  where
    T: Serialize + 'static,
  {
    let specifier = "file:///main.js".to_string();
    let module_loader =
      Rc::new(EmbeddedModuleLoader(source.to_owned(), specifier.clone()));

    let flags =
      concat!("--predictable", " --hash-seed=42", " --random-seed=42",);
    v8::V8::set_flags_from_string(flags);

    // Make's Math.random() and V8 hash seeds, address space layout repr deterministic.
    v8::V8::set_entropy_source(|buf| {
      for c in buf {
        *c = 42;
      }
      true
    });

    let params = v8::CreateParams::default().heap_limits(0, 5 << 20);
    let mut rt = JsRuntime::new(RuntimeOptions {
      extensions: vec![
        deno_webidl::init(),
        deno_url::init(),
        deno_web::init::<Permissions>(BlobStore::default(), None),
        deno_crypto::init(Some(0)),
        deno_fetch::init::<Permissions>(Options {
          user_agent: String::from("EXM"),
          ..Default::default()
        }),
        three_em_smartweave::init(arweave, op_smartweave_read_state),
        three_em_exm_base_ops::init(executor_settings.clone()),
      ],
      module_loader: Some(module_loader),
      startup_snapshot: Some(snapshot::snapshot()),
      create_params: Some(params),
      ..Default::default()
    });

    {
      let op_state = rt.op_state();
      op_state.borrow_mut().put(Permissions);
    }

    let isolate = rt.v8_isolate();

    let handle = isolate.thread_safe_handle();
    let state = Rc::new(RefCell::new(HeapLimitState::default()));

    let state_clone = state.clone();
    rt.add_near_heap_limit_callback(move |curr, _| {
      let terminated = handle.terminate_execution();
      assert!(terminated);

      *state_clone.borrow_mut() = HeapLimitState::Exceeded(curr);
      (curr + 5) << 20
    });
    /// TODO: rt.sync_ops_cache();
    let global =
      rt.execute_script("<anon>", &format!("import(\"{}\")", specifier))?;
    let module = rt.resolve_value(global).await?;

    let contract_state = {
      let scope = &mut rt.handle_scope();
      let local = serde_v8::to_v8(scope, init)?;
      v8::Global::new(scope, local)
    };

    {
      let scope = &mut rt.handle_scope();
      let context = scope.get_current_context();

      if maybe_exm_context.is_some() {
        let inner_scope = &mut v8::ContextScope::new(scope, context);

        let global = context.global(inner_scope);
        let v8_key = serde_v8::to_v8(inner_scope, "exmContext").unwrap();
        let v8_val =
          serde_v8::to_v8(inner_scope, maybe_exm_context.unwrap()).unwrap();
        global.set(inner_scope, v8_key, v8_val);
      }
    };

    {
      rt.execute_script(
        "<anon>",
        "globalThis.exmContext = Object.freeze(globalThis.exmContext);",
      );
    }

    let exm_setting_val = executor_settings
      .get("EXM")
      .unwrap_or_else(|| &Value::Bool(false));
    let is_exm = exm_setting_val.as_bool().unwrap();

    Ok(Self {
      rt,
      module,
      state,
      is_promise: None,
      contract_state,
      is_exm,
    })
  }

  pub fn state(&self) -> HeapLimitState {
    self.state.borrow().clone()
  }

  pub fn scope(&mut self) -> v8::HandleScope {
    self.rt.handle_scope()
  }

  pub fn to_value<T>(
    &mut self,
    global_value: &v8::Global<v8::Value>,
  ) -> Result<T, AnyError>
  where
    T: DeserializeOwned + 'static,
  {
    let scope = &mut self.rt.handle_scope();
    let value = v8::Local::new(scope, global_value.clone());
    Ok(serde_v8::from_v8(scope, value)?)
  }

  pub fn get_contract_state<T>(&mut self) -> Result<T, AnyError>
  where
    T: DeserializeOwned + 'static,
  {
    let scope = &mut self.rt.handle_scope();
    let value = v8::Local::new(scope, self.contract_state.clone());
    Ok(serde_v8::from_v8(scope, value)?)
  }

  pub fn get_exm_context<T>(&mut self) -> Result<T, AnyError>
  where
    T: DeserializeOwned + 'static,
  {
    let scope = &mut self.rt.handle_scope();
    let context = scope.get_current_context();
    let inner_scope = &mut v8::ContextScope::new(scope, context);
    let global = context.global(inner_scope);
    let v8_key = serde_v8::to_v8(inner_scope, "EXM").unwrap();
    let output = global.get(inner_scope, v8_key);

    if let Some(output_val) = output {
      Ok(serde_v8::from_v8(inner_scope, output_val)?)
    } else {
      Err(generic_error("Impossible to get fetch calls"))
    }
  }

  pub async fn call<R>(
    &mut self,
    action: R,
    interaction_data: Option<InteractionContext>,
  ) -> Result<Option<CallResult>, AnyError>
  where
    R: Serialize + 'static,
  {
    let global = {
      let scope = &mut self.rt.handle_scope();
      let context = scope.get_current_context();

      {
        if interaction_data.is_some() {
          let inner_scope = &mut v8::ContextScope::new(scope, context);

          let global = context.global(inner_scope);
          let v8_key =
            serde_v8::to_v8(inner_scope, "currentInteraction").unwrap();
          let v8_val =
            serde_v8::to_v8(inner_scope, interaction_data.unwrap()).unwrap();
          global.set(inner_scope, v8_key, v8_val);
        }
      };

      let action: v8::Local<v8::Value> =
        serde_v8::to_v8(scope, action).unwrap();

      let module_obj = self.module.open(scope).to_object(scope).unwrap();
      let key = v8::String::new(scope, "handle").unwrap().into();
      let func_obj = module_obj.get(scope, key).unwrap();
      let func = v8::Local::<v8::Function>::try_from(func_obj).unwrap();

      let state =
        v8::Local::<v8::Value>::new(scope, self.contract_state.clone());

      let state_clone = {
        let state_json = serde_v8::from_v8::<Value>(scope, state).unwrap();
        serde_v8::to_v8(scope, state_json).unwrap()
      };

      let undefined = v8::undefined(scope);
      let mut local = func
        .call(scope, undefined.into(), &[state_clone, action])
        .ok_or(Error::Terminated)?;

      if self.is_promise.is_none() {
        self.is_promise = Some(local.is_promise());
      }

      if let Some(true) = self.is_promise {
        let promise = v8::Local::<v8::Promise>::try_from(local).unwrap();
        match promise.state() {
          v8::PromiseState::Pending => {}
          v8::PromiseState::Fulfilled | v8::PromiseState::Rejected => {
            self.is_promise = Some(false);
            local = promise.result(scope);
          }
        }
      }

      v8::Global::new(scope, local)
    };

    let mut was_state_updated = false;

    {
      let mut result_act: Option<v8::Global<v8::Value>> = None;
      // Run the event loop.
      let global = self.rt.resolve_value(global).await?;

      // let data = self.get_contract_state::<Value>().unwrap();
      // println!("{}", data.to_string());

      let scope = &mut self.rt.handle_scope();

      let state_obj_local = v8::Local::new(scope, global).to_object(scope);

      if let Some(state) = state_obj_local {
        let state_key = v8::String::new(scope, "state").unwrap().into();

        // Return value.
        let result_key = v8::String::new(scope, "result").unwrap().into();
        let result = state.get(scope, result_key).unwrap();
        if !result.is_null_or_undefined() {
          result_act = Some(v8::Global::new(scope, result));
        }

        if let Some(state_obj) = state.get(scope, state_key) {
          if let Some(state) = state_obj.to_object(scope) {
            // Update the contract state.
            if !state_obj.is_null_or_undefined() {
              self.contract_state = v8::Global::new(scope, state_obj);
              was_state_updated = true;

              if !self.is_exm {
                // Contract evolution.
                let evolve_key =
                  v8::String::new(scope, "canEvolve").unwrap().into();
                let can_evolve = state.get(scope, evolve_key).unwrap();
                if can_evolve.boolean_value(scope) {
                  let evolve_key =
                    v8::String::new(scope, "evolve").unwrap().into();
                  let evolve = state.get(scope, evolve_key).unwrap();
                  return Ok(Some(CallResult::Evolve(
                    evolve.to_rust_string_lossy(scope),
                  )));
                }
              }
            }
          }
        }

        if let Some(result_v8_val) = result_act {
          return Ok(Some(CallResult::Result(
            result_v8_val,
            was_state_updated,
          )));
        }
      }
    };

    Ok(None)
  }
}

#[cfg(test)]
mod test {
  use crate::CallResult;
  use crate::Error;
  use crate::HeapLimitState;
  use crate::Runtime;
  use deno_core::error::AnyError;
  use deno_core::serde::Deserialize;
  use deno_core::serde::Serialize;
  use deno_core::serde_json::{json, Value};
  use deno_core::OpState;
  use deno_core::ZeroCopyBuf;
  use deno_ops::op;
  use std::cell::RefCell;
  use std::collections::HashMap;
  use std::rc::Rc;
  use three_em_exm_base_ops::ExmContext;
  use three_em_smartweave::{InteractionBlock, InteractionContext};
  use v8::Boolean;

  #[op]
  pub async fn never_op(_: (), _: (), _: ()) -> Result<Value, AnyError> {
    unreachable!()
  }

  #[tokio::test]
  async fn test_runtime() {
    let mut rt = Runtime::new(
      "export async function handle() { return { state: -69 } }",
      (),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();

    let value = rt.get_contract_state::<i32>().unwrap();
    assert_eq!(value, -69);
  }

  #[tokio::test]
  async fn test_state_empty() {
    let mut rt = Runtime::new(
      r#"export async function handle(state, action) {
        state.data++;
        state.data = Number(100 * 2 + state.data);
        if(state.data < 300) {
          return { state };
        }
      }"#,
      json!({
        "data": 0
      }),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    rt.call((), None).await.unwrap();
    let value = rt.get_contract_state::<Value>().unwrap();
    let number = value.get("data").unwrap().as_i64().unwrap();
    assert_eq!(number, 201);
  }

  #[tokio::test]
  async fn test_runtime_smartweave() {
    let mut rt = Runtime::new(
      r#"
export async function handle(slice) {
  return { state: await SmartWeave
          .arweave
          .crypto.hash(new Uint8Array(slice), 'SHA-1') }
}
"#,
      json!([0]),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let hash = rt.get_contract_state::<[u8; 20]>().unwrap();
    assert_eq!(
      hash.to_vec(),
      [
        91, 169, 60, 157, 176, 207, 249, 63, 82, 181, 33, 215, 66, 14, 67, 246,
        237, 162, 120, 79
      ]
    );
  }

  #[tokio::test]
  async fn test_runtime_smartweave_arweave_wallets_ownertoaddress() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return { state: await SmartWeave.arweave.wallets.ownerToAddress("kTuBmCmd8dbEiq4zbEPx0laVMEbgXNQ1KBUYqg3TWpLDokkcrZfa04hxYWVLZMnXH2PRSCjvCi5YVu3TG27kl29eMs-CJ-D97WyfvEZwZ7V4EDLS1uqiOrfnkBxXDfJwMI7pdGWg0JYwhsqePB8A9WfIfjrWXiGkleAAtU-dLc8Q3QYIbUBa_rNrvC_AwhXhoKUNq5gaKAdB5xQBfHJg8vMFaTsbGOxIH8v7gJyz7gc9JQf0F42ByWPmhIsm4bIHs7eGPgtUKASNBmWIgs8blP7AmbzyJp4bx_AOQ4KOCei25Smw2-UAZehCGibl50i-blv5ldpGhcKDBC7ukjZpOY99V0mdDynbQBi606DdTWGJSXGNkvpwYnLh53VOE3uX0zuxNnRlwA9BN_VisWMrQwk_KnB0Fz0qGlJsXNQEWb_TEaf6eWLcSIUZUUC9o0L6J6mI9hiJjf_sisiR6AsWF4UoA-snWsFNzgPdkeOHW_biJMep6DOnWX8lmh8meDGMi1XOxJ4hJAawD7uS3A8jL7Kn7eYtiQ7bnZG69WtBueyOQh78yStMvoKz6awzBt1IaTBUG9_CHrEy_Tx6aQZu1c2D_nZonTd0pV2ljC7E642VtOWsRFL78-1xF6P0FD4eWh6HoDpD05_3oUBrAdusLMkn8Gm5tl0wIwMrLF58FYk") }
}
"#,
      json!([0]),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
        .await
        .unwrap();

    rt.call((), None).await.unwrap();
    let address = rt.get_contract_state::<String>().unwrap();
    assert_eq!(address, "z5-Ql2zU5Voac97BHMDGrk3_2gEDqM72iHxCJhkcJ5A");
  }

  #[tokio::test]
  async fn test_runtime_smartweave_crypto_sign() {
    let mut rt = Runtime::new(
      r#"
export async function handle(slice) {
  try {
    const jwk = {
      kty: 'RSA',
      n: 'kTuBmCmd8dbEiq4zbEPx0laVMEbgXNQ1KBUYqg3TWpLDokkcrZfa04hxYWVLZMnXH2PRSCjvCi5YVu3TG27kl29eMs-CJ-D97WyfvEZwZ7V4EDLS1uqiOrfnkBxXDfJwMI7pdGWg0JYwhsqePB8A9WfIfjrWXiGkleAAtU-dLc8Q3QYIbUBa_rNrvC_AwhXhoKUNq5gaKAdB5xQBfHJg8vMFaTsbGOxIH8v7gJyz7gc9JQf0F42ByWPmhIsm4bIHs7eGPgtUKASNBmWIgs8blP7AmbzyJp4bx_AOQ4KOCei25Smw2-UAZehCGibl50i-blv5ldpGhcKDBC7ukjZpOY99V0mdDynbQBi606DdTWGJSXGNkvpwYnLh53VOE3uX0zuxNnRlwA9BN_VisWMrQwk_KnB0Fz0qGlJsXNQEWb_TEaf6eWLcSIUZUUC9o0L6J6mI9hiJjf_sisiR6AsWF4UoA-snWsFNzgPdkeOHW_biJMep6DOnWX8lmh8meDGMi1XOxJ4hJAawD7uS3A8jL7Kn7eYtiQ7bnZG69WtBueyOQh78yStMvoKz6awzBt1IaTBUG9_CHrEy_Tx6aQZu1c2D_nZonTd0pV2ljC7E642VtOWsRFL78-1xF6P0FD4eWh6HoDpD05_3oUBrAdusLMkn8Gm5tl0wIwMrLF58FYk',
      e: 'AQAB',
      d: 'XKMTT8bD-32dkkP5gwZ32k3mDYw4Ep49ZdrHB7mX5f8VkI-IHmZta15ty8074QcqE9isppWNm_Xh3VkHvkjmwH2GHWzlPaCy993AqexYSJ6k_dgdSn8RidjCeNbK5JeO3jpaSSeGA2a5f1EAy6KPDvnrFjFbiWF2RS9D5GLrBEw_Gmx9tYpGQI6bmsbu8h3Y9IozhQ-ZJ40xiT7mj8W5d15yRiQwbZ5Rhw6q1uedkafGZbeEB_34GkiBwmusGmxfo0_d7fd176yvc7QR9jY7BrfUjHvMDbvuRoMl5gQBq-pntxb3u9t_fIFAoMPNA9EPvv8l3WMEds-SmHmDLXpNdTbIXn6yguGSs9Lci0o7jjLCigOX0qu73UqSuCbXY0TE39s4bAoFWFVcaIgyHWMkbt6BV_OERhbsU5K47NYRg__BUEr39ruG3BnuvWJFwIeLGp5OUDlvsvWQn9VkOSXNJi7kvrVucwwT95vYvGtgoQnU5csIIo66ciyvCatjVUy7YLS8kdoKjRdu57wQJXUsrH5PXgUnomIGO8NCrf0WB5XBFaPL8m5_nDs4_Ym_gD7A5rR-S0OHGDF6L4xDcStvmpeqHEmF1o872vKeayXi23pfsFWfpLM1WnuFcIGuqxjT6TQQZFL1Z-LwEQp5RyvnF8SBapLMJiQYXOcm0M8K2-0',
      p: 'wNeunobSmEgjFw1uNyWMsXtCBFNQDs_XY1oYMq6S_Y9d6AQ0cVx7TFjikUb4ipzIenUc28PlAAGe1c7E6WjcSbIrcyiTT_vkSy6KJznlRYOMZkckRnkvm7f7w80OfSrb4kSUyyXhlL0XfH_WjG9CMGbwoA5MM-3NEyUCJ04cFBtCQC2Lx-lcT30HZKbjVCblVG9zNqu3FePcz90zKpxno6z9Ie9zkmO1xPjFNlUug3NFGj8GOVrii4PxXDIycinUv08zcxY5z9XqD3vUYk84N5JgGoHBsQ1BdbU2naGJ374RXueYb3Ogx-4wYfzp7l_CPqsQCcL9HEGKsM2QzVXniw',
      q: 'wMwY5tm_Jj4V9eYQ_UgfWZkqtLzzhqVU_VFZ1G_6s36OGpSVevQKcEQpvFnCphihDzthW4N5sSyO0eBgwHdbuQ4tkS-iSNsKnASQVT81yQUanslI11-259L1aUNIJynAqSXFcoNPhyUMrouOR4bYCCFqnyXlpxSWg2zYQUDJnG5Uv_wGR5zizVAgYeWJyvlwUxBJIJUCaLbNs0hKK09OZ0Z0C7WSCcAKwDW6LK3KWZfFGedaMMQQTWCBpK14Y1WYAN67t7I9dEHimZI5jSNRmi2FX4NXrhNuORk20hcPT4s4Fdvuwu7yrlhg_5VOr7IpZY8mXtUgwmVHBFFGNko5uw',
      dp: 'kEMJY5hShQ86CO3ILMMPbFpb-aZltp7vb2ifv5JvbfZJdt9maAOaTXQVEj84gWFmbI2d6B20-3s62pHTJxWF7i-2Z3DMO0Kh90g6m7uo84bEimLgFURlRCWv1ztYgnSEh9FsSkjtZ3rJzh5IX0iACHuJuQLZKOPVzWObJ9I8GSKHPkGUVxoRL3nGBRr_5x0t5Ct30kdFMMAEmQ_OTisxMPWhbDiYicPD4DWGOu4gXL_nywmo21FNNree4KzApjz65Z8XSxouZ3eMoMavDFhdIt2CvXGid5QGC0tkLyoAXXvvvMKee4nRlp9uXG96hRPn2T_ZQKQ4-2FgooE1uRZxnw',
      dq: 'uVe8HLl57G7FN9bLwGJEWSNJDeWUC34HnVtGi1Z3YXUpcV4j8caH_nNY2AxGdty4gOcp6gsTwwK97f_Ro1VbZSS_I5LyZS3GHkS46GrS7wQsGjgRAZOvR1_jsyUOSS_3WeTI0xRvMNGqRmY9CoAUUISndoW9KAk_xOqvXtPEvdDHQqUq-E9XLd94sgQzmmB_3iqK0nrNjRMn3tGBE--yxM_TIaqU0TDAZRWBfBA6tjSUNBnX94eU0H4VQ9XMJVqUvUlilu8P6yKnj9El6Ivql9hpHnAqq1tcnCGkNQYcHvEMot8Cwn1p6bdm0G2d7oPNDig2z_X9_0PTqM_lOq3Snw',
      qi: 'WA7rs38z_LZad6SFGJNUblyuJ-W7zFkFtHqh8_ToUVbS6wuNLbmsOr5_AsOWKWKils2eHWj5bA4Io5SajWl499JgGLS7nMwhn1gSzIfYskoHCl4_isEu7mB2uOWqPtSt6xYvCaxutyTQSbaUj9ioOsOU5Gjt-Vuigm3M5rmQS6Kli1rPgs1boYj8NPtou21SwrHXZnsfA2J7QqzDddhhLdd8U85_H4eFygiSwYbnnIkMSciWt6CAviPve-MeQMIKKtATjIUspUzBlbCuHR7WaMqyVvYfCRhsDg8WaRIsgebz4qSUwzuy-Lip8EFXcMbzocjP5JHE4eKFm5H9Iq0V5Q'
    };
    const data = new TextEncoder().encode('Hello');
    const sign = await SmartWeave.arweave.crypto.sign(jwk, data);
    const verify = await SmartWeave.arweave.crypto.verify(jwk.n, data, sign);
    return {
      state: verify
    }
  } catch(e) {
    return { state: e.stack }
  }
}
"#,
      json!([0]),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
        .await
        .unwrap();

    rt.call((), None).await.unwrap();
    let hash = rt.get_contract_state::<Value>().unwrap();
    assert_eq!(hash, Value::Bool(true));
  }

  #[tokio::test]
  async fn test_runtime_date() {
    let mut executor_settings: HashMap<String, Value> = HashMap::new();
    executor_settings.insert(
      String::from("TX_DATE"),
      Value::String(String::from("1662327465259")),
    );
    executor_settings.insert(
      String::from("EXM"),
      deno_core::serde_json::Value::Bool(true),
    );
    let mut rt = Runtime::new(
      r#"
export async function handle(slice) {
  try {
    return {
      state: EXM.getDate().getTime()
    }
  } catch(e) {
    return { state: e.stack }
  }
}
"#,
      json!([0]),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      executor_settings,
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let hash = rt.get_contract_state::<usize>().unwrap();
    assert_eq!(hash, 1662327465259 as usize);
  }

  #[tokio::test]
  async fn test_runtime_vanilla_date() {
    let mut executor_settings: HashMap<String, Value> = HashMap::new();
    let mut rt = Runtime::new(
      r#"
export async function handle(slice) {
  try {
    return {
      state: new Date().getTime()
    }
  } catch(e) {
    return { state: e.stack }
  }
}
"#,
      json!([0]),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      executor_settings,
      None,
    )
    .await
    .unwrap();

    rt.call(
      (),
      Some(InteractionContext {
        transaction: Default::default(),
        block: InteractionBlock {
          timestamp: 1662327465200 as usize,
          height: 0 as usize,
          indep_hash: String::new(),
        },
      }),
    )
    .await
    .unwrap();
    let hash = rt.get_contract_state::<usize>().unwrap();
    assert_eq!(hash, 1662327465200 as usize);
  }

  #[tokio::test]
  async fn test_base_fetch_op() {
    let mut exec_settings: HashMap<String, deno_core::serde_json::Value> =
      HashMap::new();
    exec_settings.insert(
      String::from("EXM"),
      deno_core::serde_json::Value::Bool(true),
    );
    let mut rt = Runtime::new(
      r#"
export async function handle() {
try {
  const someFetch = await EXM.deterministicFetch("https://arweave.net/tx/YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI");
  const someFetch2 = await EXM.deterministicFetch("https://arweave.net/tx/RjOdIx9Y42f0T19-Tm_xB2Nk_blBv56eJ14tfXMNZTg");
  return { state: someFetch.asJSON().id };
  } catch(e) {
  return { state: e.toString() }
  }
}
"#,
      (),
      (12345, String::from("arweave.net"), String::from("http")),
      never_op::decl(),
      exec_settings,
      None
    )
        .await
        .unwrap();

    rt.call((), None).await.unwrap();
    let calls = rt.get_exm_context::<ExmContext>().unwrap();

    println!("{}", deno_core::serde_json::to_string(&calls).unwrap());
    let tx_id = rt.get_contract_state::<String>().unwrap();
    assert_eq!(
      tx_id.to_string(),
      "YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI"
    );
    assert_eq!(calls.requests.keys().len(), 2);
    assert_eq!(
      calls
        .requests
        .get(
          "7c13bc2cb63b30754ee3047ca46337e626d61d01b8484ecea8d3e235a617091a"
            .into()
        )
        .unwrap()
        .url,
      "https://arweave.net/tx/YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI"
    );
  }

  #[tokio::test]
  async fn test_base_fetch_op_with_context() {
    let mut exec_settings: HashMap<String, deno_core::serde_json::Value> =
      HashMap::new();
    exec_settings.insert(
      String::from("EXM"),
      deno_core::serde_json::Value::Bool(true),
    );
    exec_settings.insert(
      String::from("LAZY_EVALUATION"),
      deno_core::serde_json::Value::Bool(true),
    );

    let exm_context_from: ExmContext = deno_core::serde_json::from_str(r#"{"requests":{"7c13bc2cb63b30754ee3047ca46337e626d61d01b8484ecea8d3e235a617091a":{"type":"basic","url":"https://arweave.net/tx/YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI","statusText":"OK","status":127,"redirected":false,"ok":true,"headers":{"content-type":"application/json; charset=utf-8","vary":"Origin","x-amz-cf-id":"C09sMjl6s7bG2A3a-t9_Ydci7sh3dul3mfnoOtI_pHdbGJTzwk1OOQ==","x-cache":"Miss from cloudfront","date":"Wed, 24 Aug 2022 19:36:22 GMT","x-trace":"tXKxF8qQt","server":"envoy","cache-control":"public, max-age=31536000","x-amz-cf-pop":"MIA3-C5","x-envoy-upstream-service-time":"10","via":"1.1 4de3cdbf8046367453bc168e829b445e.cloudfront.net (CloudFront)"},"vector":[123,34,102,111,114,109,97,116,34,58,50,44,34,105,100,34,58,34,89,117,74,118,67,74,69,77,105,107,48,74,52,81,81,106,90,85,76,67,97,69,106,105,102,65,66,75,89,104,45,104,69,90,80,72,57,122,111,107,79,119,73,34,44,34,108,97,115,116,95,116,120,34,58,34,72,97,90,80,117,121,85,73,86,51,77,54,101,116,103,103,97,98,76,105,101,122,57,83,75,113,118,76,71,95,70,83,57,76,109,110,54,106,76,66,50,76,79,52,90,65,74,76,107,72,103,51,104,54,89,95,48,110,90,86,66,45,118,95,34,44,34,111,119,110,101,114,34,58,34,115,119,100,53,68,120,121,52,103,86,86,56,76,90,95,89,87,65,87,57,105,69,89,100,83,78,72,120,106,78,84,73,117,49,90,90,45,57,116,110,85,85,66,45,80,89,109,50,65,111,78,113,56,55,53,118,48,75,98,80,90,107,57,88,105,52,48,103,73,111,54,89,111,88,79,49,53,55,106,67,119,49,84,101,79,89,104,75,117,97,52,106,103,86,50,55,74,105,98,67,103,115,57,101,103,66,71,81,102,89,67,107,52,118,65,77,50,102,57,71,45,55,48,72,103,56,77,45,106,98,74,73,77,95,67,56,48,80,101,50,98,73,51,53,113,52,70,81,83,119,89,76,111,76,45,103,67,82,83,83,98,73,66,51,100,71,67,89,67,118,78,88,122,121,84,57,110,83,119,111,108,119,84,99,45,77,120,74,90,74,116,104,72,102,87,117,53,49,54,70,53,69,100,82,67,76,122,103,97,83,109,110,50,65,69,56,74,56,82,69,117,54,113,68,52,81,80,68,69,69,117,50,55,108,71,86,114,49,119,48,101,70,86,72,104,54,49,77,101,111,69,107,57,119,116,77,120,45,78,117,89,74,95,107,52,53,119,71,51,121,119,122,73,121,100,54,105,57,48,111,118,72,74,45,97,89,75,80,80,109,122,85,105,106,98,116,54,101,66,45,117,121,51,85,114,109,105,68,82,118,84,103,57,57,51,75,112,87,118,53,70,114,69,54,56,122,108,116,99,57,105,115,122,74,119,70,51,86,110,113,71,98,87,67,67,67,51,118,83,86,103,86,111,48,113,74,67,106,97,88,57,50,84,70,82,102,111,73,52,52,118,65,107,77,45,83,98,52,45,108,111,74,106,70,68,57,69,110,114,86,115,71,53,113,87,81,103,86,106,53,114,106,86,56,89,102,65,101,66,113,121,50,70,112,103,107,104,52,50,111,55,85,115,110,98,69,51,105,79,75,104,111,104,90,81,97,48,84,118,49,106,80,56,95,83,81,101,119,55,72,90,55,110,74,112,72,102,106,86,111,84,98,76,54,89,116,109,69,116,110,109,112,90,119,107,74,76,65,118,81,75,99,76,107,53,114,106,74,98,120,90,48,86,70,49,51,102,45,77,108,85,76,120,102,98,70,104,97,77,106,49,77,66,80,50,82,76,76,85,100,50,70,55,115,51,111,81,69,76,104,66,53,83,98,83,98,68,50,111,90,118,95,67,118,116,100,76,102,111,77,101,82,80,104,90,99,53,79,66,67,85,111,88,49,119,51,81,112,105,114,100,88,49,97,51,75,87,98,99,56,115,57,106,117,120,68,87,107,114,104,114,109,120,73,97,48,54,55,116,50,106,82,97,66,55,87,115,74,83,55,73,117,98,49,100,100,97,103,104,109,74,68,109,114,110,113,55,98,111,65,90,53,115,71,99,67,103,75,87,97,56,114,51,101,71,75,109,76,67,69,77,114,105,88,75,73,117,101,119,67,120,65,118,81,120,108,80,51,107,122,52,114,65,104,72,45,108,81,49,110,75,71,56,34,44,34,116,97,103,115,34,58,91,123,34,110,97,109,101,34,58,34,81,50,57,117,100,72,74,104,89,51,81,116,85,51,74,106,34,44,34,118,97,108,117,101,34,58,34,85,109,112,80,90,69,108,52,79,86,107,48,77,109,89,119,86,68,69,53,76,86,82,116,88,51,104,67,77,107,53,114,88,50,74,115,81,110,89,49,78,109,86,75,77,84,82,48,90,108,104,78,84,108,112,85,90,119,34,125,93,44,34,116,97,114,103,101,116,34,58,34,34,44,34,113,117,97,110,116,105,116,121,34,58,34,48,34,44,34,100,97,116,97,34,58,34,34,44,34,100,97,116,97,95,115,105,122,101,34,58,34,49,34,44,34,100,97,116,97,95,116,114,101,101,34,58,91,93,44,34,100,97,116,97,95,114,111,111,116,34,58,34,100,89,70,68,71,48,120,95,122,103,85,107,102,76,90,77,99,112,116,99,109,82,50,122,104,101,49,103,103,88,52,118,48,97,85,73,106,101,74,121,106,100,103,34,44,34,114,101,119,97,114,100,34,58,34,54,54,56,54,57,50,48,56,34,44,34,115,105,103,110,97,116,117,114,101,34,58,34,97,82,100,79,121,75,109,83,49,122,83,67,79,116,99,89,104,112,107,115,75,121,88,53,66,105,71,106,100,79,75,50,111,88,71,101,110,72,104,75,67,68,104,67,103,77,110,80,72,90,120,100,121,107,81,78,95,118,76,90,73,77,117,90,66,104,75,66,87,70,70,88,78,104,85,107,75,89,52,82,116,108,116,114,81,111,115,114,117,101,108,76,70,120,73,73,57,71,53,122,105,69,48,117,66,102,109,73,55,86,90,103,67,85,105,71,70,67,122,116,83,88,57,74,80,48,68,106,71,105,95,56,87,45,104,115,117,98,97,71,65,76,115,107,71,50,113,51,104,79,67,54,121,73,113,57,50,79,105,83,54,120,68,102,103,74,104,111,69,65,67,118,118,71,85,112,84,102,72,100,53,122,67,48,120,108,45,53,119,99,85,86,90,114,86,68,113,121,82,85,121,77,54,103,82,67,52,76,80,77,108,49,57,56,113,102,114,116,95,101,56,98,71,107,102,86,84,121,53,68,97,54,117,65,49,53,51,102,99,48,115,70,117,78,88,116,73,81,57,77,51,72,118,116,66,50,87,118,87,68,67,55,99,78,75,100,71,111,67,101,99,69,90,66,105,89,116,80,54,102,76,53,87,114,99,54,57,50,115,75,72,68,49,106,69,98,122,115,55,113,70,82,88,70,115,48,48,112,101,117,50,111,53,56,95,82,83,71,65,122,105,104,121,117,67,71,115,67,52,119,111,90,48,117,99,54,48,104,69,104,80,53,55,84,97,99,69,120,121,121,53,100,113,113,85,50,113,66,86,76,98,77,54,103,69,100,118,81,66,103,109,108,116,82,114,111,48,110,120,75,79,107,109,52,104,105,70,112,111,86,121,89,81,104,67,66,85,50,101,84,89,88,86,105,79,67,87,57,56,86,85,57,81,98,57,107,74,68,52,87,121,119,53,52,111,120,89,48,68,100,110,66,119,87,110,98,50,49,112,102,48,115,45,45,114,89,82,116,118,83,71,85,117,50,75,83,89,97,83,73,68,110,113,65,52,99,81,66,90,77,116,118,69,51,84,52,105,89,114,81,84,50,106,121,112,109,80,55,107,77,104,118,110,105,68,85,89,48,112,73,101,118,50,54,103,74,122,45,89,97,118,107,87,79,79,48,111,117,82,75,76,77,111,85,115,81,95,79,100,80,73,76,85,104,85,75,122,90,75,70,78,112,115,54,77,95,53,82,86,89,68,49,97,110,117,74,89,83,115,79,56,78,89,99,118,103,83,106,114,118,49,118,119,105,108,111,116,75,49,78,82,106,81,120,54,95,66,50,65,88,119,113,66,72,69,66,107,118,105,110,113,90,76,99,45,110,105,122,70,82,89,48,53,50,84,113,66,102,108,88,55,66,52,57,104,90,98,121,106,104,67,49,71,87,90,90,105,82,70,89,45,106,76,107,86,77,82,121,110,111,79,76,88,116,115,73,48,66,80,88,88,73,99,122,80,113,122,99,120,119,102,111,102,102,102,50,111,101,71,73,50,70,81,34,125]},"db2d50c0afb58537f2007535e4c357540cb20fbfb15e73e7f48cc03260ed0596":{"type":"basic","url":"https://arweave.net/tx/RjOdIx9Y42f0T19-Tm_xB2Nk_blBv56eJ14tfXMNZTg","statusText":"OK","status":127,"redirected":false,"ok":true,"headers":{"cache-control":"public, max-age=31536000","date":"Wed, 24 Aug 2022 19:36:22 GMT","via":"1.1 4de3cdbf8046367453bc168e829b445e.cloudfront.net (CloudFront)","x-amz-cf-id":"tB_17HcetObNsKbfdmiAky3zQB18tIBAHOeJjzrsta1yShmVZ1vJSQ==","x-cache":"Miss from cloudfront","x-envoy-upstream-service-time":"18","x-trace":"WV_BWwIxp","server":"envoy","x-amz-cf-pop":"MIA3-C5","content-type":"application/json; charset=utf-8","vary":"Origin"},"vector":[123,34,102,111,114,109,97,116,34,58,50,44,34,105,100,34,58,34,82,106,79,100,73,120,57,89,52,50,102,48,84,49,57,45,84,109,95,120,66,50,78,107,95,98,108,66,118,53,54,101,74,49,52,116,102,88,77,78,90,84,103,34,44,34,108,97,115,116,95,116,120,34,58,34,72,97,90,80,117,121,85,73,86,51,77,54,101,116,103,103,97,98,76,105,101,122,57,83,75,113,118,76,71,95,70,83,57,76,109,110,54,106,76,66,50,76,79,52,90,65,74,76,107,72,103,51,104,54,89,95,48,110,90,86,66,45,118,95,34,44,34,111,119,110,101,114,34,58,34,115,119,100,53,68,120,121,52,103,86,86,56,76,90,95,89,87,65,87,57,105,69,89,100,83,78,72,120,106,78,84,73,117,49,90,90,45,57,116,110,85,85,66,45,80,89,109,50,65,111,78,113,56,55,53,118,48,75,98,80,90,107,57,88,105,52,48,103,73,111,54,89,111,88,79,49,53,55,106,67,119,49,84,101,79,89,104,75,117,97,52,106,103,86,50,55,74,105,98,67,103,115,57,101,103,66,71,81,102,89,67,107,52,118,65,77,50,102,57,71,45,55,48,72,103,56,77,45,106,98,74,73,77,95,67,56,48,80,101,50,98,73,51,53,113,52,70,81,83,119,89,76,111,76,45,103,67,82,83,83,98,73,66,51,100,71,67,89,67,118,78,88,122,121,84,57,110,83,119,111,108,119,84,99,45,77,120,74,90,74,116,104,72,102,87,117,53,49,54,70,53,69,100,82,67,76,122,103,97,83,109,110,50,65,69,56,74,56,82,69,117,54,113,68,52,81,80,68,69,69,117,50,55,108,71,86,114,49,119,48,101,70,86,72,104,54,49,77,101,111,69,107,57,119,116,77,120,45,78,117,89,74,95,107,52,53,119,71,51,121,119,122,73,121,100,54,105,57,48,111,118,72,74,45,97,89,75,80,80,109,122,85,105,106,98,116,54,101,66,45,117,121,51,85,114,109,105,68,82,118,84,103,57,57,51,75,112,87,118,53,70,114,69,54,56,122,108,116,99,57,105,115,122,74,119,70,51,86,110,113,71,98,87,67,67,67,51,118,83,86,103,86,111,48,113,74,67,106,97,88,57,50,84,70,82,102,111,73,52,52,118,65,107,77,45,83,98,52,45,108,111,74,106,70,68,57,69,110,114,86,115,71,53,113,87,81,103,86,106,53,114,106,86,56,89,102,65,101,66,113,121,50,70,112,103,107,104,52,50,111,55,85,115,110,98,69,51,105,79,75,104,111,104,90,81,97,48,84,118,49,106,80,56,95,83,81,101,119,55,72,90,55,110,74,112,72,102,106,86,111,84,98,76,54,89,116,109,69,116,110,109,112,90,119,107,74,76,65,118,81,75,99,76,107,53,114,106,74,98,120,90,48,86,70,49,51,102,45,77,108,85,76,120,102,98,70,104,97,77,106,49,77,66,80,50,82,76,76,85,100,50,70,55,115,51,111,81,69,76,104,66,53,83,98,83,98,68,50,111,90,118,95,67,118,116,100,76,102,111,77,101,82,80,104,90,99,53,79,66,67,85,111,88,49,119,51,81,112,105,114,100,88,49,97,51,75,87,98,99,56,115,57,106,117,120,68,87,107,114,104,114,109,120,73,97,48,54,55,116,50,106,82,97,66,55,87,115,74,83,55,73,117,98,49,100,100,97,103,104,109,74,68,109,114,110,113,55,98,111,65,90,53,115,71,99,67,103,75,87,97,56,114,51,101,71,75,109,76,67,69,77,114,105,88,75,73,117,101,119,67,120,65,118,81,120,108,80,51,107,122,52,114,65,104,72,45,108,81,49,110,75,71,56,34,44,34,116,97,103,115,34,58,91,123,34,110,97,109,101,34,58,34,81,50,57,117,100,71,86,117,100,67,49,85,101,88,66,108,34,44,34,118,97,108,117,101,34,58,34,89,88,66,119,98,71,108,106,89,88,82,112,98,50,52,118,97,109,70,50,89,88,78,106,99,109,108,119,100,65,34,125,44,123,34,110,97,109,101,34,58,34,81,88,66,119,76,85,53,104,98,87,85,34,44,34,118,97,108,117,101,34,58,34,82,85,48,34,125,44,123,34,110,97,109,101,34,58,34,86,72,108,119,90,81,34,44,34,118,97,108,117,101,34,58,34,85,50,86,121,100,109,86,121,98,71,86,122,99,119,34,125,93,44,34,116,97,114,103,101,116,34,58,34,34,44,34,113,117,97,110,116,105,116,121,34,58,34,48,34,44,34,100,97,116,97,34,58,34,34,44,34,100,97,116,97,95,115,105,122,101,34,58,34,57,52,34,44,34,100,97,116,97,95,116,114,101,101,34,58,91,93,44,34,100,97,116,97,95,114,111,111,116,34,58,34,78,52,101,79,71,74,105,89,95,73,106,106,122,78,45,69,51,52,49,49,102,112,86,76,117,66,109,117,71,71,115,114,65,65,120,77,78,99,119,75,105,77,81,34,44,34,114,101,119,97,114,100,34,58,34,54,54,56,54,57,50,48,56,34,44,34,115,105,103,110,97,116,117,114,101,34,58,34,73,48,102,115,70,54,90,67,77,67,78,108,89,75,99,90,75,118,65,98,121,100,112,51,117,69,79,83,68,104,97,50,116,116,48,50,98,49,73,78,85,110,79,95,55,118,69,99,101,55,109,84,75,55,66,82,65,73,88,86,77,50,87,98,66,114,48,86,99,56,84,80,116,115,74,50,77,74,86,110,103,103,99,78,53,100,106,106,48,102,68,90,48,80,112,118,121,45,98,109,73,70,55,102,118,50,118,113,86,79,119,105,82,45,82,117,55,57,77,71,88,71,95,87,99,85,53,71,107,114,106,53,79,102,66,112,113,54,89,68,111,72,116,83,100,70,115,117,88,81,121,104,107,116,103,111,104,106,105,108,89,82,50,69,110,84,89,84,105,71,74,113,117,77,108,75,83,70,119,70,113,103,108,73,114,88,98,121,122,56,74,57,90,95,83,110,56,119,101,55,104,114,89,68,105,76,45,75,45,84,115,69,95,115,45,95,51,79,70,70,70,118,48,101,108,116,100,48,68,110,100,45,50,78,118,71,50,52,108,75,65,80,112,116,55,104,55,120,102,98,69,83,101,87,82,75,74,85,84,79,79,53,109,99,73,97,103,56,110,57,106,112,51,75,72,95,79,69,114,52,49,74,45,50,50,104,103,74,105,49,82,97,106,78,48,65,82,56,111,83,108,119,98,99,115,85,55,85,102,90,49,119,113,55,120,107,72,69,120,85,115,73,102,86,48,70,114,116,101,116,57,111,89,114,79,80,77,97,118,74,86,68,69,70,88,72,90,82,72,99,79,119,68,69,102,66,79,78,57,103,88,75,73,83,53,119,97,111,74,103,71,110,111,69,67,66,118,54,79,98,52,85,88,65,114,79,111,104,98,76,85,49,85,100,89,87,120,100,100,56,88,101,114,113,74,121,98,85,102,110,107,69,95,110,111,84,86,57,72,110,98,114,110,116,119,48,53,68,52,56,76,109,52,72,77,105,119,115,75,122,74,108,80,107,108,116,53,90,83,73,116,45,68,101,107,99,90,82,112,77,81,122,116,49,105,121,56,111,89,81,80,55,112,82,103,87,122,75,69,70,56,114,100,101,109,119,105,90,87,85,114,120,97,49,100,113,83,88,67,95,79,53,119,85,95,107,51,71,98,51,104,116,99,45,73,87,109,86,114,115,56,82,119,65,102,52,90,48,75,117,117,120,100,80,108,122,107,54,79,50,95,113,73,71,121,52,87,114,51,79,68,105,69,112,53,80,107,105,116,66,89,115,95,52,49,74,76,113,77,79,109,98,102,110,121,121,113,118,82,98,116,120,76,106,114,54,53,98,111,81,98,86,50,109,48,111,101,54,50,104,57,55,104,116,72,110,79,85,109,67,65,48,116,66,70,114,109,52,51,101,107,112,83,119,54,67,83,117,48,103,120,72,104,53,86,71,45,81,121,68,108,116,87,119,57,103,73,65,54,65,57,90,88,70,102,103,82,90,50,72,121,50,55,84,86,50,87,67,77,65,56,102,66,52,45,98,110,82,73,103,113,82,119,34,125]}}}"#).unwrap();

    let mut rt = Runtime::new(
      r#"
export async function handle() {
try {
  const someFetch = await EXM.deterministicFetch("https://arweave.net/tx/YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI");
  return { state: [someFetch.asJSON().id, String(someFetch.raw.length), someFetch.url]  };
  } catch(e) {
  return { state: e.toString() }
  }
}
"#,
      (),
      (12345, String::from("arweave.net"), String::from("http")),
      never_op::decl(),
      exec_settings,
      Some(deno_core::serde_json::to_value(exm_context_from).unwrap())
    )
        .await
        .unwrap();

    rt.call((), None).await.unwrap();
    let calls = rt.get_exm_context::<ExmContext>().unwrap();

    let state = rt.get_contract_state::<Vec<String>>().unwrap();
    assert_eq!(
      state.get(0).unwrap().to_string(),
      "YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI"
    );
    assert_eq!(state.get(1).unwrap().to_string(), "1784");
    assert_eq!(
      state.get(2).unwrap().to_string(),
      "https://arweave.net/tx/YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI"
    );
    assert_eq!(calls.requests.keys().len(), 0);
  }

  #[tokio::test]
  async fn test_deterministic_v8() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return { state: Math.random() };
}
"#,
      (),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let rand1 = rt.get_contract_state::<f64>().unwrap();
    assert_eq!(rand1, 0.3800000002095474);

    rt.call((), None).await.unwrap();
    let rand2 = rt.get_contract_state::<f64>().unwrap();
    assert_eq!(rand2, 0.1933761369163034);
  }

  #[tokio::test]
  async fn test_deterministic_crypto_random() {
    let mut rt = Runtime::new(
      r#"
  export async function handle(size) {
    const u8 = new Uint8Array(size);
    await crypto.getRandomValues(u8);
    return { state: Object.values(u8) };
  }
  "#,
      8,
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let rand1 = rt.get_contract_state::<[u8; 8]>().unwrap();
    assert_eq!(rand1.as_ref(), &[127, 111, 44, 205, 178, 63, 42, 187]);

    rt.call((), None).await.unwrap();
    let rand2 = rt.get_contract_state::<[u8; 8]>().unwrap();
    assert_eq!(rand2.as_ref(), &[123, 105, 39, 142, 148, 124, 1, 198]);
  }

  #[tokio::test]
  async fn test_deterministic_gc() {
    let mut rt = Runtime::new(
      r#"
  let called = false;
  const registry = new FinalizationRegistry((_) => {
    called = true;
  });

  export async function handle() {
    let x = new Uint8Array(1024 * 1024);
    registry.register(x, "called!");
    x = null;
    return { state: called };
  }
  "#,
      (),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call(&(), None).await.unwrap();
    let gced = rt.get_contract_state::<bool>().unwrap();
    assert_eq!(gced, false);
  }

  #[tokio::test]
  async fn test_deterministic_weakref() {
    let mut rt = Runtime::new(
      r#"
  export async function handle() {
    let obj = { value: true };
    const weakRef = new WeakRef(obj);
    {
      const wrapper = (_) => { return weakRef.deref()?.value };
    }
    return { state: weakRef.deref()?.value || false };
  }
  "#,
      (),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let exists = rt.get_contract_state::<bool>().unwrap();
    assert_eq!(exists, true);
  }

  #[tokio::test]
  async fn test_deterministic_allocation_failure() {
    let mut rt = Runtime::new(
        r#"
  export async function handle() {
    return { state: "Hello, World!".repeat(1024 * 1024 * 5).split("").reverse().join("") };
  }
  "#,
  (),
        (80, String::from("arweave.net"), String::from("https")),
        never_op::decl(),
        HashMap::new(),
      None
      )
      .await
      .unwrap();

    let err = rt
      .call((), None)
      .await
      .unwrap_err()
      .downcast::<Error>()
      .unwrap();
    assert_eq!(err, Error::Terminated);

    match rt.state() {
      HeapLimitState::Exceeded(_current) => {}
      _ => panic!("Expected heap limit to be exceeded"),
    }
  }

  #[tokio::test]
  async fn test_contract_evolve() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return { state: { canEvolve: true, evolve: "xxdummy" } };
}"#,
      (),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    let evolved = rt.call((), None).await.unwrap();
    assert_eq!(evolved, Some(CallResult::Evolve("xxdummy".to_string())));
  }

  #[tokio::test]
  async fn test_smartweave_host_ops() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return { state: await SMARTWEAVE_HOST() };
}
"#,
      (),
      (12345, String::from("arweave.net"), String::from("http")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let host = rt.get_contract_state::<String>().unwrap();
    assert_eq!(host, "http://arweave.net:12345");
  }

  #[tokio::test]
  async fn test_smartweave_get_tx() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  try {
  const tx = await SmartWeave.unsafeClient.transactions.get("1OLypVtx3fIh-zq6iAihS0I1HBMwDp-fm_3kuGLDOTY");
  const currTag = tx.get("tags")[0];
    return {
      state: [
        currTag.get("name", { decode: true, string: true }),
        currTag.get("value", { decode: true, string: true }),
        tx.get("id")
      ]
    };
  } catch(e) {
    return { state: [e.stack.toString(), "", ""] }
  }
}
"#,
      (),
      (443, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None
    )
        .await
        .unwrap();

    rt.call((), None).await.unwrap();
    let data = rt.get_contract_state::<Vec<String>>().unwrap();
    assert_eq!(data.get(0).unwrap(), "App-Name");
    assert_eq!(data.get(1).unwrap(), "SmartWeaveContract");
    assert_eq!(
      data.get(2).unwrap(),
      "1OLypVtx3fIh-zq6iAihS0I1HBMwDp-fm_3kuGLDOTY"
    );
  }

  #[tokio::test]
  async fn test_op_settings() {
    let mut settings: HashMap<String, deno_core::serde_json::Value> =
      HashMap::new();
    settings.insert(
      String::from("Country"),
      deno_core::serde_json::Value::String("United States".to_string()),
    );
    settings.insert(
      String::from("Simulated"),
      deno_core::serde_json::Value::Bool(true),
    );
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return {
    state: [Deno.core.opSync("op_get_executor_settings", "Country"), Deno.core.opSync("op_get_executor_settings", "Simulated"), Deno.core.opSync("op_get_executor_settings", "unknown")]
  }
}
"#,
      (),
      (443, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      settings,
      None
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let data = rt
      .get_contract_state::<(String, bool, deno_core::serde_json::Value)>()
      .unwrap();
    assert_eq!(data.0, "United States");
    assert_eq!(data.1, true);
    assert_eq!(data.2, deno_core::serde_json::Value::Null);
  }

  #[derive(Serialize, Deserialize)]
  struct GetDataTest {
    data: String,
    data2: String,
  }

  #[tokio::test]
  async fn test_smartweave_ops_get_data() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
try {
  return { state: [await SmartWeave.unsafeClient.transactions.getData("YzVdaDBnaiGToFQJAnJCGtyJwJZbaCASotWEPFhBgBY"),
                  await SmartWeave.unsafeClient.transactions.getData("YzVdaDBnaiGToFQJAnJCGtyJwJZbaCASotWEPFhBgBY", { decode: true, string: true })]
  };
  } catch(error) {
    return { state: [error.toString()] }
  }
}
"#,
      (),
      (443, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None
    )
        .await
        .unwrap();

    let mut interaction_data = InteractionContext {
      transaction: Default::default(),
      block: Default::default(),
    };
    interaction_data.transaction.id =
      String::from("YzVdaDBnaiGToFQJAnJCGtyJwJZbaCASotWEPFhBgBY");

    rt.call((), Some(interaction_data)).await.unwrap();
    let host = rt.get_contract_state::<Vec<String>>().unwrap();
    assert_eq!(host.get(0).unwrap().to_owned(), String::from("eyJjb250ZW50Ijp7ImJvZHkiOiJoZWxsbyB3b3JsZCIsInRpbWVzdGFtcCI6MTY0MTYzNDQzOCwidGl0bGUiOiJoZWxsbyB3b3JsZCJ9LCJkaWdlc3QiOiJOLVJ6UmZXMmxzV0RqdV9GcE5RUzZhWmdzTzdma1Z5eVo3bWJzRWhpNjZ3IiwiYXV0aG9yc2hpcCI6eyJjb250cmlidXRvciI6IjB4ZWFlYjIyNjIwREJEOTgwRTc0NjNjOWQ5NkE1YWU0ZDk0NDhiMTMzYyIsInNpZ25pbmdLZXkiOiJ7XCJjcnZcIjpcIlAtMjU2XCIsXCJleHRcIjp0cnVlLFwia2V5X29wc1wiOltcInZlcmlmeVwiXSxcImt0eVwiOlwiRUNcIixcInhcIjpcImRVcEI3MVhJV1lZYjBQSGlqdkJicTNtMl9CNFA2aGZFZHNBdndkS0JDNk1cIixcInlcIjpcIlBHRmlveWtaODU4cHN3cEZXODZtdjZKQlBFZ1Y4WFNVZWY5M3pqNUFoNFFcIn0iLCJzaWduYXR1cmUiOiJ1WE9YWVJrX1dFVmdaM0xOR0xhWUVsNmVGYS1xa1JyWURwRjJ5ektwUXJhS1I1R0lqWEdiSXg4QUFZQ0VPNEZEQXhVeF93bjVkUWR1RldZZTNKTHEtUSIsInNpZ25pbmdLZXlTaWduYXR1cmUiOiIweDhhMmFjNTE3NTg5OTA2NDkwMWEzZTBlM2VjODc0ZjZmOTFjMWE3NzVjODM4NjNmMDY2OWE0YjUxMjhiYjgxODcwYTQzMmUwZTM1YjAwMjUxZTdmMTg0ZTRlYzE2ZTNjOWVkNDM0YjBjMjkzMDc3N2I3M2UzMzg3MDk5MWQ0NjhlMWIiLCJzaWduaW5nS2V5TWVzc2FnZSI6IkkgYXV0aG9yaXplIHB1Ymxpc2hpbmcgb24gbWlycm9yLnh5eiBmcm9tIHRoaXMgZGV2aWNlIHVzaW5nOlxue1wiY3J2XCI6XCJQLTI1NlwiLFwiZXh0XCI6dHJ1ZSxcImtleV9vcHNcIjpbXCJ2ZXJpZnlcIl0sXCJrdHlcIjpcIkVDXCIsXCJ4XCI6XCJkVXBCNzFYSVdZWWIwUEhpanZCYnEzbTJfQjRQNmhmRWRzQXZ3ZEtCQzZNXCIsXCJ5XCI6XCJQR0Zpb3lrWjg1OHBzd3BGVzg2bXY2SkJQRWdWOFhTVWVmOTN6ajVBaDRRXCJ9IiwiYWxnb3JpdGhtIjp7Im5hbWUiOiJFQ0RTQSIsImhhc2giOiJTSEEtMjU2In19LCJuZnQiOnt9LCJ2ZXJzaW9uIjoiMTItMjEtMjAyMCIsIm9yaWdpbmFsRGlnZXN0IjoiTi1SelJmVzJsc1dEanVfRnBOUVM2YVpnc083ZmtWeXlaN21ic0VoaTY2dyJ9"));
    assert_eq!(
      host.get(1).unwrap().to_owned(),
      include_str!("../../testdata/test_smartweave_ops_get_data.json").trim()
    );
  }

  #[tokio::test]
  async fn test_contract_result() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return { result: "Hello, World!" };
}"#,
      (),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    let result = rt
      .call((), None)
      .await
      .unwrap()
      .expect("Expected CallResult");

    match result {
      CallResult::Result(value, state_updated) => {
        let scope = &mut rt.scope();
        let local = v8::Local::new(scope, value);
        let value: String = deno_core::serde_v8::from_v8(scope, local).unwrap();
        assert_eq!(value, "Hello, World!".to_string());
        assert_eq!(state_updated, false);
      }
      CallResult::Evolve(evolve) => panic!(
        "Expected CallResult::Result, got CallResult::Evolve({})",
        evolve
      ),
    }
  }

  #[tokio::test]
  async fn test_contract_state_updated() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return { result: "Hello, World!", state: 1230 };
}"#,
      (),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
      None,
    )
    .await
    .unwrap();

    let result = rt
      .call((), None)
      .await
      .unwrap()
      .expect("Expected CallResult");

    match result {
      CallResult::Result(value, state_updated) => {
        let scope = &mut rt.scope();
        let local = v8::Local::new(scope, value);
        let value: String = deno_core::serde_v8::from_v8(scope, local).unwrap();
        assert_eq!(value, "Hello, World!".to_string());
        assert_eq!(state_updated, true);
      }
      CallResult::Evolve(evolve) => panic!(
        "Expected CallResult::Result, got CallResult::Evolve({})",
        evolve
      ),
    }
  }
}
