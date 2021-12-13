mod loader;
pub mod snapshot;

use crate::loader::EmbeddedModuleLoader;
use deno_core::error::AnyError;
use deno_core::serde::de::DeserializeOwned;
use deno_core::serde::Serialize;
use deno_core::serde_v8;
use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use deno_web::BlobStore;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use three_em_smartweave::ContractInfo;

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
    contract_info: ContractInfo,
  ) -> Result<Self, AnyError>
  where
    T: Serialize + 'static,
  {
    let specifier = "file:///main.js".to_string();
    let module_loader =
      Rc::new(EmbeddedModuleLoader(source.to_owned(), specifier.clone()));

    let flags = concat!(
      "--predictable",
      " --predictable-gc-schedule",
      " --hash-seed=42",
      " --random-seed=42",
    );
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
        deno_web::init(BlobStore::default(), None),
        deno_crypto::init(Some(0)),
        three_em_smartweave::init(contract_info),
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
      curr + 5 << 20
    });
    rt.sync_ops_cache();

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

  pub fn get_contract_state<T>(&mut self) -> Result<T, AnyError>
  where
    T: DeserializeOwned + 'static,
  {
    let scope = &mut self.rt.handle_scope();
    let value = v8::Local::new(scope, self.contract_state.clone());
    Ok(serde_v8::from_v8(scope, value)?)
  }

  pub async fn call<R>(&mut self, action: R) -> Result<Option<String>, AnyError>
  where
    R: Serialize + 'static,
  {
    let global = {
      let scope = &mut self.rt.handle_scope();
      let action: v8::Local<v8::Value> =
        serde_v8::to_v8(scope, action).unwrap();

      let module_obj = self.module.open(scope).to_object(scope).unwrap();
      let key = v8::String::new(scope, "handle").unwrap().into();
      let func_obj = module_obj.get(scope, key).unwrap();
      let func = v8::Local::<v8::Function>::try_from(func_obj)?;

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
      let state_obj = state.get(scope, state_key).unwrap();
      self.contract_state = v8::Global::new(scope, state_obj);

      if let Some(state) = state_obj.to_object(scope) {
        let evolve_key = v8::String::new(scope, "canEvolve").unwrap().into();
        let can_evolve = state.get(scope, evolve_key).unwrap();
        if can_evolve.boolean_value(scope) {
          let evolve_key = v8::String::new(scope, "evolve").unwrap().into();
          let evolve = state.get(scope, evolve_key).unwrap();
          return Ok(Some(evolve.to_rust_string_lossy(scope)));
        }
      }
    };

    Ok(None)
  }
}

#[cfg(test)]
mod test {
  use crate::Error;
  use crate::HeapLimitState;
  use crate::Runtime;
  use deno_core::ZeroCopyBuf;
  use three_em_smartweave::ContractInfo;

  #[tokio::test]
  async fn test_runtime() {
    let mut rt = Runtime::new(
      "export async function handle() { return { state: -69 } }",
      (),
      ContractInfo::default(),
    )
    .await
    .unwrap();

    rt.call(()).await.unwrap();

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
      ContractInfo::default(),
    )
    .await
    .unwrap();

    rt.call(()).await.unwrap();
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
  async fn test_deterministic_v8() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {
  return { state: Math.random() };
}
"#,
      (),
      ContractInfo::default(),
    )
    .await
    .unwrap();

    rt.call(()).await.unwrap();
    let rand1 = rt.get_contract_state::<f64>().unwrap();
    assert_eq!(rand1, 0.3800000002095474);

    rt.call(()).await.unwrap();
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
      ContractInfo::default(),
    )
    .await
    .unwrap();

    rt.call(()).await.unwrap();
    let rand1 = rt.get_contract_state::<[u8; 8]>().unwrap();
    assert_eq!(rand1.as_ref(), &[127, 111, 44, 205, 178, 63, 42, 187]);

    rt.call(()).await.unwrap();
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
      ContractInfo::default(),
    )
    .await
    .unwrap();

    rt.call(&()).await.unwrap();
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
      ContractInfo::default(),
    )
    .await
    .unwrap();

    rt.call(()).await.unwrap();
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
  ContractInfo::default(),
      )
      .await
      .unwrap();

    let err = rt.call(()).await.unwrap_err().downcast::<Error>().unwrap();
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
      ContractInfo::default(),
    )
    .await
    .unwrap();

    let evolved = rt.call(()).await.unwrap();
    assert_eq!(evolved, Some("xxdummy".to_string()));
  }
}
