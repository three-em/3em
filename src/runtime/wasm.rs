use deno_core::error::AnyError;
use deno_core::JsRuntime;

pub struct WasmRuntime {
  rt: JsRuntime,
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
      let v8_module = rt.execute_script("<anon>", "new WebAssembly.Module(WASM_BINARY)").unwrap();

      let scope = &mut rt.handle_scope();
      let global = global.open(scope).global(scope);
      let v8_module =  v8::Local::new(scope, v8_module);
      let name = v8::String::new(scope, "WASM_MODULE").unwrap();
      global.set(scope, name.into(), v8_module);
    }

    Ok(Self { rt })
  }
}

#[cfg(test)]
mod tests {
  use crate::runtime::wasm;

  #[tokio::test]
  async fn test_wasm_runtime() {
    let _rt = wasm::WasmRuntime::new(&[0; 100]);
  }
}
