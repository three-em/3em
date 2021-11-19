use deno_core::error::AnyError;
use deno_core::JsRuntime;
use deno_core::ZeroCopyBuf;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::Cell;
use std::fmt::Debug;

pub struct WasmRuntime {
  rt: JsRuntime,
  handle: v8::Global<v8::Function>,
  allocator: v8::Global<v8::Function>,
  result_len: v8::Global<v8::Function>,
  exports: v8::Global<v8::Object>,
}

impl WasmRuntime {
  pub async fn new(wasm: &[u8]) -> Result<WasmRuntime, AnyError> {
    let mut rt = JsRuntime::new(Default::default());

    // Get hold of the WebAssembly object.
    let wasm_obj = rt.execute_script("<anon>", "WebAssembly").unwrap();
    let (exports, handle, allocator, result_len) = {
      let scope = &mut rt.handle_scope();
      let buf =
        v8::ArrayBuffer::new_backing_store_from_boxed_slice(wasm.into());
      let buf = v8::SharedRef::from(buf);
      let buf = v8::ArrayBuffer::with_backing_store(scope, &buf);

      let wasm_obj = wasm_obj.get(scope).to_object(scope).unwrap();

      // Create a new WebAssembly.Instance object.
      let module_str = v8::String::new(scope, "Module").unwrap();
      let module_value = wasm_obj.get(scope, module_str.into()).unwrap();
      let module_constructor =
        v8::Local::<v8::Function>::try_from(module_value)?;

      let module = module_constructor
        .new_instance(scope, &[buf.into()])
        .unwrap();

      // Create a new WebAssembly.Instance object.
      let instance_str = v8::String::new(scope, "Instance").unwrap();
      let instance_value = wasm_obj.get(scope, instance_str.into()).unwrap();
      let instance_constructor =
        v8::Local::<v8::Function>::try_from(instance_value)?;

      let imports = v8::Object::new(scope);
      // AssemblyScript needs `abort` to be defined.
      let env = v8::Object::new(scope);
      let abort_str = v8::String::new(scope, "abort").unwrap();
      let function_callback =
        |_: &mut v8::HandleScope,
         _: v8::FunctionCallbackArguments,
         _: v8::ReturnValue| {
          // No-op.
        };

      let abort_callback = v8::Function::new(scope, function_callback).unwrap();
      env.set(scope, abort_str.into(), abort_callback.into());

      let env_str = v8::String::new(scope, "env").unwrap();
      imports.set(scope, env_str.into(), env.into());

      let instance = instance_constructor
        .new_instance(scope, &[module.into(), imports.into()])
        .unwrap();

      let exports_str = v8::String::new(scope, "exports").unwrap();
      let exports = instance.get(scope, exports_str.into()).unwrap();
      let exports = v8::Local::<v8::Object>::try_from(exports)?;

      let alloc_str = v8::String::new(scope, "_alloc").unwrap();
      let alloc_obj = exports.get(scope, alloc_str.into()).unwrap();
      let allocator = v8::Local::<v8::Function>::try_from(alloc_obj)?;
      let allocator = v8::Global::new(scope, allocator);

      let handle_str = v8::String::new(scope, "handle").unwrap();

      let handle_obj = exports.get(scope, handle_str.into()).unwrap();
      let handle = v8::Local::<v8::Function>::try_from(handle_obj)?;
      let handle = v8::Global::new(scope, handle);

      let result_len_str = v8::String::new(scope, "get_len").unwrap();

      let result_len_obj = exports.get(scope, result_len_str.into()).unwrap();
      let result_len = v8::Local::<v8::Function>::try_from(result_len_obj)?;
      let result_len = v8::Global::new(scope, result_len);

      let exports = v8::Global::new(scope, exports);
      (exports, handle, allocator, result_len)
    };

    Ok(Self {
      rt,
      handle,
      allocator,
      result_len,
      exports,
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

      let exports_obj = self.exports.get(scope).to_object(scope).unwrap();

      let mem_str = v8::String::new(scope, "memory").unwrap();
      let mem_obj = exports_obj.get(scope, mem_str.into()).unwrap();
      let mem_obj = v8::Local::<v8::Object>::try_from(mem_obj)?;

      let buffer_str = v8::String::new(scope, "buffer").unwrap();
      let buffer_obj = mem_obj.get(scope, buffer_str.into()).unwrap();

      let mem_buf = v8::Local::<v8::ArrayBuffer>::try_from(buffer_obj)?;

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

    for i in 1..100 {
      let mut prev_state_bytes =
        deno_core::serde_json::to_vec(&prev_state).unwrap();
      let state = rt.call(&mut prev_state_bytes).await.unwrap();

      let state: Value = deno_core::serde_json::from_slice(&state).unwrap();

      assert_eq!(state.get("counter").unwrap(), i);
      prev_state = state;
    }
  }

  #[tokio::test]
  async fn test_wasm_runtime_asc() {
    let mut rt =
      wasm::WasmRuntime::new(include_bytes!("./testdata/02_wasm/02_wasm.wasm"))
        .await
        .unwrap();

    let mut prev_state = json!({
      "counter": 0,
    });

    for i in 1..100 {
      let mut prev_state_bytes =
        deno_core::serde_json::to_vec(&prev_state).unwrap();
      let state = rt.call(&mut prev_state_bytes).await.unwrap();

      let state: Value = deno_core::serde_json::from_slice(&state).unwrap();

      assert_eq!(state.get("counter").unwrap(), i);
      prev_state = state;
    }
  }
}
