pub mod extensions;

mod module_loader;
mod smartweave;

use crate::runtime::extensions::get_extensions;
use crate::runtime::module_loader::EmbeddedModuleLoader;
use crate::snapshot_js::three_em_isolate;
use deno_core::error::AnyError;
use deno_core::serde::de::DeserializeOwned;
use deno_core::serde::Serialize;
use deno_core::RuntimeOptions;
use deno_core::{Extension, JsRuntime};
use deno_web::BlobStore;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
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

    let mut rt = JsRuntime::new(RuntimeOptions {
      // TODO(@littledivy): Move this to snapshots
      extensions: get_extensions(),
      module_loader: Some(module_loader),
      startup_snapshot: Some(three_em_isolate()),
      ..Default::default()
    });

    rt.sync_ops_cache();

    let global =
      rt.execute_script("<anon>", &format!("import(\"{}\")", specifier))?;
    let module = rt.resolve_value(global).await?;

    Ok(Self { rt, module })
  }

  pub async fn call<R, T>(&mut self, arguments: &[R]) -> Result<T, AnyError>
  where
    R: Serialize + 'static,
    T: Debug + DeserializeOwned + 'static,
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
}
