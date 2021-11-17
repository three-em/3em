use deno_core::error::AnyError;
use deno_core::JsRuntime;
use deno_core::ZeroCopyBuf;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::Cell;
use std::fmt::Debug;

pub struct WasmRuntime {
  rt: JsRuntime,
  handle: v8::Global<v8::Value>,
  allocator: v8::Global<v8::Value>,
}

impl WasmRuntime {
  pub async fn new(wasm: &[u8]) -> Result<WasmRuntime, AnyError> {
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

    let allocator = rt
      .execute_script("<anon>", "WASM_INSTANCE.exports._alloc")
      .unwrap();
    

    Ok(Self {
      rt,
      handle,
      allocator,
    })
  }

  pub async fn call<T>(&mut self, state: &mut [u8]) -> Result<T, AnyError>
  where
    T: Debug + DeserializeOwned + 'static,
  {

    let global = {
      let scope = &mut self.rt.handle_scope();
      let undefined = v8::undefined(scope);

      let alloc_obj = self.allocator.get(scope).to_object(scope).unwrap();
      let alloc = v8::Local::<v8::Function>::try_from(alloc_obj)?;

      let state_len = v8::Number::new(scope, state.len() as f64);

      // Offset in memory for start of the block.
      let local_ptr = alloc
        .call(scope, undefined.into(), &[state_len.into()])
        .unwrap();
      let local_ptr_u32 = local_ptr.uint32_value(scope).unwrap();
      
      let source = v8::String::new(scope, "WASM_INSTANCE.exports.memory.buffer").unwrap();
      let script = v8::Script::compile(scope, source, None).unwrap();
      let mem = script.run(scope).unwrap();
      let mem = v8::Global::new(scope, mem);
      let mem_obj = mem.get(scope).to_object(scope).unwrap();
      let mem_buf = v8::Local::<v8::ArrayBuffer>::try_from(mem_obj).unwrap();

      // O HOLY Backin' store.
      let store = mem_buf.get_backing_store();

      let mut raw_mem = unsafe {
        get_backing_store_slice_mut(&store, local_ptr_u32 as usize, state.len())
      };

      assert_eq!(raw_mem.len(), state.len());
      raw_mem.swap_with_slice(state);

      let module_obj = self.handle.get(scope).to_object(scope).unwrap();
      let func = v8::Local::<v8::Function>::try_from(module_obj)?;

      let result_ptr = func
        .call(
          scope,
          undefined.into(),
          &[local_ptr, state_len.into(), local_ptr, state_len.into()],
        )
        .unwrap();
            
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

unsafe fn get_backing_store_slice_mut(
  backing_store: &v8::SharedRef<v8::BackingStore>,
  byte_offset: usize,
  byte_length: usize,
) -> &mut [u8] {
  let cells: *const [Cell<u8>] =
    &backing_store[byte_offset..byte_offset + byte_length];
  let bytes = cells as *const _ as *mut [u8];
  &mut *bytes
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

    let initial_state = json!({
      "counter": 0,
    });

    let mut initial_state_bytes =
      deno_core::serde_json::to_vec(&initial_state).unwrap();
    let state: Value = rt.call(&mut initial_state_bytes).await.unwrap();
    panic!("{:#?}", state);
  }
}
