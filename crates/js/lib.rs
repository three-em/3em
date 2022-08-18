pub mod default_permissions;
mod loader;
pub mod snapshot;

use crate::default_permissions::Permissions;
use crate::loader::EmbeddedModuleLoader;
use deno_core::error::AnyError;
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
  Result(v8::Global<v8::Value>),
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
}

impl Runtime {
  pub async fn new<T>(
    source: &str,
    init: T,
    arweave: (i32, String, String),
    op_smartweave_read_state: OpDecl,
    executor_settings: HashMap<String, deno_core::serde_json::Value>,
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
        deno_tls::init(),
        deno_fetch::init::<Permissions>(Options {
          user_agent: String::from("EXM"),
          ..Default::default()
        }),
        three_em_smartweave::init(arweave, op_smartweave_read_state),
        three_em_exm_base_ops::init(executor_settings),
      ],
      module_loader: Some(module_loader),
      startup_snapshot: Some(snapshot::snapshot()),
      create_params: Some(params),
      ..Default::default()
    });
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

    Ok(Self {
      rt,
      module,
      state,
      is_promise: None,
      contract_state,
    })
  }

  pub fn state(&self) -> HeapLimitState {
    self.state.borrow().clone()
  }

  pub fn scope(&mut self) -> v8::HandleScope {
    self.rt.handle_scope()
  }

  pub fn get_contract_state<T>(&mut self) -> Result<T, AnyError>
  where
    T: DeserializeOwned + 'static,
  {
    let scope = &mut self.rt.handle_scope();
    let value = v8::Local::new(scope, self.contract_state.clone());
    Ok(serde_v8::from_v8(scope, value)?)
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
      let undefined = v8::undefined(scope);
      let mut local = func
        .call(scope, undefined.into(), &[state, action])
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

    {
      // Run the event loop.
      let global = self.rt.resolve_value(global).await?;

      let scope = &mut self.rt.handle_scope();

      let state = v8::Local::new(scope, global)
        .to_object(scope)
        .ok_or(Error::Terminated)?;
      let state_key = v8::String::new(scope, "state").unwrap().into();

      // Return value.
      let result_key = v8::String::new(scope, "result").unwrap().into();
      let result = state.get(scope, result_key).unwrap();
      if !result.is_null_or_undefined() {
        return Ok(Some(CallResult::Result(v8::Global::new(scope, result))));
      }

      let state_obj = state.get(scope, state_key).unwrap();
      if let Some(state) = state_obj.to_object(scope) {
        // Update the contract state.
        self.contract_state = v8::Global::new(scope, state_obj);

        // Contract evolution.
        let evolve_key = v8::String::new(scope, "canEvolve").unwrap().into();
        let can_evolve = state.get(scope, evolve_key).unwrap();
        if can_evolve.boolean_value(scope) {
          let evolve_key = v8::String::new(scope, "evolve").unwrap().into();
          let evolve = state.get(scope, evolve_key).unwrap();
          return Ok(Some(CallResult::Evolve(
            evolve.to_rust_string_lossy(scope),
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
  use deno_core::serde_json::Value;
  use deno_core::OpState;
  use deno_core::ZeroCopyBuf;
  use deno_ops::op;
  use std::cell::RefCell;
  use std::collections::HashMap;
  use std::rc::Rc;
  use three_em_smartweave::InteractionContext;

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
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();

    let value = rt.get_contract_state::<i32>().unwrap();
    assert_eq!(value, -69);
  }

  #[tokio::test]
  async fn test_runtime_smartweave() {
    let buf: Vec<u8> = vec![0x00];
    let mut rt = Runtime::new(
      r#"
export async function handle(slice) {
  return { state: await SmartWeave
          .arweave
          .crypto.hash(slice, 'SHA-1') }
}
"#,
      ZeroCopyBuf::from(buf),
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
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
  async fn test_base_fetch_op() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
try {
  const someFetch = await fetch("https://arweave.net/tx/YuJvCJEMik0J4QQjZULCaEjifABKYh-hEZPH9zokOwI");
  return { state: `Hello` };
  } catch(e) {
  return { state: e.toString() }
  }
}
"#,
      (),
      (12345, String::from("arweave.net"), String::from("http")),
      never_op::decl(),
      HashMap::new(),
    )
        .await
        .unwrap();

    rt.call((), None).await.unwrap();
    let tx_id = rt
      .get_contract_state::<deno_core::serde_json::Value>()
      .unwrap();
    println!("{}", tx_id)
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
    return { state: u8 };
  }
  "#,
      8,
      (80, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      HashMap::new(),
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
        HashMap::new()
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
      HashMap::new()
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
    state: [await Deno.core.opAsync("op_get_executor_settings", "Country"), await Deno.core.opAsync("op_get_executor_settings", "Simulated")]
  }
}
"#,
      (),
      (443, String::from("arweave.net"), String::from("https")),
      never_op::decl(),
      settings,
    )
    .await
    .unwrap();

    rt.call((), None).await.unwrap();
    let data = rt.get_contract_state::<(String, bool)>().unwrap();
    assert_eq!(data.0, "United States");
    assert_eq!(data.1, true);
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
      HashMap::new()
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
    )
    .await
    .unwrap();

    let result = rt
      .call((), None)
      .await
      .unwrap()
      .expect("Expected CallResult");

    match result {
      CallResult::Result(value) => {
        let scope = &mut rt.scope();
        let local = v8::Local::new(scope, value);
        let value: String = deno_core::serde_v8::from_v8(scope, local).unwrap();
        assert_eq!(value, "Hello, World!".to_string());
      }
      CallResult::Evolve(evolve) => panic!(
        "Expected CallResult::Result, got CallResult::Evolve({})",
        evolve
      ),
    }
  }
}
