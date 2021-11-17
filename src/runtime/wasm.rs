use deno_core::error::AnyError;
use deno_core::JsRuntime;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

pub struct WasmRuntime {
  rt: JsRuntime,
  handle: v8::Global<v8::Value>,
}

impl WasmRuntime {
  pub async fn new(wasm: &[u8]) -> Result<Self, AnyError> {
    let mut rt = JsRuntime::new(Default::default());
    let global = rt.global_context();
    {
      let scope = &mut rt.handle_scope();
      let global = global.open(scope).global(scope);
      let buf =
        v8::ArrayBuffer::new_backing_store_from_boxed_slice(wasm.into());
      let buf = v8::SharedRef::from(buf);
      let name = v8::String::new(scope, "WASM_BINARY").unwrap();
      let buf = v8::ArrayBuffer::with_backing_store(scope, &buf);
      global.set(scope, name.into(), buf.into());
    }

    {
      let v8_module = rt
        .execute_script("<anon>", "new WebAssembly.Module(WASM_BINARY)")
        .unwrap();

      let scope = &mut rt.handle_scope();
      let global = global.open(scope).global(scope);
      let v8_module = v8::Local::new(scope, v8_module);
      let name = v8::String::new(scope, "WASM_MODULE").unwrap();
      global.set(scope, name.into(), v8_module);
    }

    {
      let v8_instance = rt
        .execute_script("<anon>", "new WebAssembly.Instance(WASM_MODULE)")
        .unwrap();

      let scope = &mut rt.handle_scope();

      let global = global.open(scope).global(scope);
      let v8_instance = v8::Local::new(scope, v8_instance);

      let name = v8::String::new(scope, "WASM_INSTANCE").unwrap();
      global.set(scope, name.into(), v8_instance);

      let name = v8::String::new(scope, "EXPORT_NAME").unwrap();
      let func_name = v8::String::new(scope, "handle").unwrap();
      global.set(scope, name.into(), func_name.into());
    }

    let handle = rt
      .execute_script("<anon>", "WASM_INSTANCE.exports[EXPORT_NAME]")
      .unwrap();

    Ok(Self { rt, handle })
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
      let module_obj = self.handle.get(scope).to_object(scope).unwrap();
      let func = v8::Local::<v8::Function>::try_from(module_obj)?;

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
mod tests {
  use crate::runtime::wasm;
  use deno_core::serde_json::json;
  use deno_core::serde_json::Value;

  #[tokio::test]
  async fn test_wasm_runtime() {
    let _rt = wasm::WasmRuntime::new(&[0; 100]);
  }

  #[tokio::test]
  async fn test_wasm_runtime_contract() {
    let mut rt =
      wasm::WasmRuntime::new(include_bytes!("./testdata/01_wasm/01_wasm.wasm"))
        .await
        .unwrap();

    let state: Value = rt
      .call(&[json!({
        "counter": 0,
      })])
      .await
      .unwrap();
  }
}
