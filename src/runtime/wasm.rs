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
  result_len: v8::Global<v8::Value>,
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
    }

    let handle = rt
      .execute_script("<anon>", "WASM_INSTANCE.exports.handle")
      .unwrap();

    let allocator = rt
      .execute_script("<anon>", "WASM_INSTANCE.exports._alloc")
      .unwrap();

    let result_len = rt
      .execute_script("<anon>", "WASM_INSTANCE.exports.get_len")
      .unwrap();

    Ok(Self {
      rt,
      handle,
      allocator,
      result_len,
    })
  }

  pub async fn call(&mut self, state: &mut [u8]) -> Result<Vec<u8>, AnyError> {
    let result = {
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

      let source =
        v8::String::new(scope, "WASM_INSTANCE.exports.memory.buffer").unwrap();
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

      let handler_obj = self.handle.get(scope).to_object(scope).unwrap();
      let handle = v8::Local::<v8::Function>::try_from(handler_obj)?;

      let result_ptr = handle
        .call(
          scope,
          undefined.into(),
          &[local_ptr, state_len.into(), local_ptr, state_len.into()],
        )
        .unwrap();
      let result_ptr_u32 = result_ptr.uint32_value(scope).unwrap();
      let get_len_obj = self.result_len.get(scope).to_object(scope).unwrap();
      let get_len = v8::Local::<v8::Function>::try_from(get_len_obj)?;

      let result_len = get_len.call(scope, undefined.into(), &[]).unwrap();
      let result_len = result_len.uint32_value(scope).unwrap();

      let mut result_mem = unsafe {
        get_backing_store_slice_mut(
          &store,
          result_ptr_u32 as usize,
          result_len as usize,
        )
      };

      assert_eq!(result_mem.len(), result_len as usize);

      result_mem.to_vec()
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

    let mut prev_state = json!({
      "counter": 0,
    });

    // Hundred thousand interactions
    for i in 1..100_000 {
      let mut prev_state_bytes =
        deno_core::serde_json::to_vec(&prev_state).unwrap();
      let state = rt.call(&mut prev_state_bytes).await.unwrap();

      let state: Value = deno_core::serde_json::from_slice(&state).unwrap();

      assert_eq!(state.get("counter").unwrap(), i);
      prev_state = state;
    }
  }
}
