mod module_loader;
mod smartweave;
mod snapshot;
mod wasm;

use crate::runtime::module_loader::EmbeddedModuleLoader;
use deno_core::error::AnyError;
use deno_core::serde::de::DeserializeOwned;
use deno_core::serde::Serialize;
use deno_core::RuntimeOptions;
use deno_core::Extension;
use deno_core::JsRuntime;
use deno_web::BlobStore;
use std::fmt::Debug;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

pub struct Runtime {
  rt: JsRuntime,
  module: v8::Global<v8::Value>,
}

impl Runtime {
  pub async fn new(source: &str) -> Result<Self, AnyError> {
    let specifier = "file:///main.js".to_string();

    let module_loader =
      Rc::new(EmbeddedModuleLoader(source.to_owned(), specifier.clone()));

    let flags = concat!("--predictable", " --predictable-gc-schedule", " --hash-seed=42");
    v8::V8::set_flags_from_string(flags);

    // Make's Math.random() and V8 hash seeds, address space layout repr deterministic.
    v8::V8::set_entropy_source(|buf| {
      for c in buf {
        *c = 42;
      }
      true
    });
    
    let params = v8::CreateParams::default().heap_limits(0, 10 << 20);
    let mut rt = JsRuntime::new(RuntimeOptions {
      extensions: vec![
        deno_webidl::init(),
        deno_url::init(),
        deno_web::init(BlobStore::default(), None),
        deno_crypto::init(Some(0)),
        smartweave::init(),
      ],
      module_loader: Some(module_loader),
      startup_snapshot: Some(snapshot::snapshot()),
      create_params: Some(params),
      ..Default::default()
    });
    
    fn heap_limit_reached(current: usize, initial: usize) -> usize {
        println!("{} {}", current, initial);
        current
    }
    rt.add_near_heap_limit_callback(heap_limit_reached);
    rt.sync_ops_cache();
    let global =
      rt.execute_script("<anon>", &format!("import(\"{}\")", specifier))?;
    let module = rt.resolve_value(global).await?;

    Ok(Self { rt, module })
  }

  pub async fn call<R, T>(&mut self, arguments: &[R]) -> Result<T, AnyError>
  where
    R: Serialize + 'static,
    T: DeserializeOwned + 'static,
  {
    let global = {
      let scope = &mut self.rt.handle_scope();
      let arguments: Vec<v8::Local<v8::Value>> = arguments
        .iter()
        .map(|argument| deno_core::serde_v8::to_v8(scope, argument).unwrap())
        .collect();

      let module_obj = self.module.open(scope).to_object(scope).unwrap();
      let key = v8::String::new(scope, "handle").unwrap().into();
      let func_obj = module_obj.get(scope, key).unwrap();
      let func = v8::Local::<v8::Function>::try_from(func_obj)?;

      let undefined = v8::undefined(scope);
      let local = func.call(scope, undefined.into(), &arguments).unwrap();
      v8::Global::new(scope, local)
    };

    let result: T = {
      // Run the event loop.
      let value = self.rt.resolve_value(global).await?;
      let scope = &mut self.rt.handle_scope();

      let value = v8::Local::new(scope, value);
      deno_core::serde_v8::from_v8(scope, value)?
    };

    Ok(result)
  }
}

#[cfg(test)]
mod test {
  use crate::runtime::Runtime;
  use deno_core::ZeroCopyBuf;

  #[tokio::test]
  async fn test_runtime() {
    let mut rt = Runtime::new("export async function handle() { return -69 }")
      .await
      .unwrap();
    let value: i64 = rt.call(&[()]).await.unwrap();

    assert_eq!(value, -69);
  }

  #[tokio::test]
  async fn test_runtime_smartweave() {
    let mut rt = Runtime::new(
      r#"
export async function handle(slice) {
  return SmartWeave
          .arweave
          .crypto.hash(slice, 'SHA-1') 
}
"#,
    )
    .await
    .unwrap();

    let buf: Vec<u8> = vec![0x00];
    let hash: [u8; 20] = rt.call(&[ZeroCopyBuf::from(buf)]).await.unwrap();
    assert_eq!(
      hash,
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
  return Math.random();
}
"#,
    )
    .await
    .unwrap();

    let rand1: f64 = rt.call(&[()]).await.unwrap();
    let rand2: f64 = rt.call(&[()]).await.unwrap();

    assert_eq!(rand1, 0.14617804087311326);
    assert_eq!(rand2, 0.16993119449737915);
  }

  #[tokio::test]
  async fn test_deterministic_crypto_random() {
    let mut rt = Runtime::new(
      r#"
export async function handle(size) {
  const u8 = new Uint8Array(size);
  await crypto.getRandomValues(u8);
  return u8; 
}
"#,
    )
    .await
    .unwrap();

    let rand1: ZeroCopyBuf = rt.call(&[8]).await.unwrap();
    let rand2: ZeroCopyBuf = rt.call(&[8]).await.unwrap();

    assert_eq!(rand1.as_ref(), &[127, 111, 44, 205, 178, 63, 42, 187]);
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
  let x = new Uint8Array(1024 * 1024 * 100);
  registry.register(x, "called!");
  x = null;
  return called;
}
"#,
    )
    .await
    .unwrap();

    let gced: bool = rt.call(&[()]).await.unwrap();
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
  return weakRef.deref()?.value || false;
}
"#,
    )
    .await
    .unwrap();

    let exists: bool = rt.call(&[()]).await.unwrap();
    assert_eq!(exists, true);
  }

  #[tokio::test]
  async fn test_deterministic_allocation_failure() {
    let mut rt = Runtime::new(
      r#"
export async function handle() {  
  const size = 1024 * 1024 * 100;
  const array = new Uint8Array(size);
  for (let i = 0; i < size; i += 4096) {
    array[i] = 1;
  }

  return array;
}
"#,
    )
    .await
    .unwrap();

    let exists: bool = rt.call(&[()]).await.unwrap();
    assert_eq!(exists, true);
  }
}
