use crate::runtime::smartweave;
use crate::runtime::smartweave::ContractInfo;
use deno_core::error::AnyError;
use deno_core::JsRuntime;
use deno_core::ZeroCopyBuf;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::Cell;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

macro_rules! wasm_alloc {
  ($scope: expr, $alloc: expr, $this: expr, $len: expr) => {
    $alloc.call($scope, $this.into(), &[$len.into()]).unwrap()
  };
}

static COST: AtomicUsize = AtomicUsize::new(0);

pub struct WasmRuntime {
  rt: JsRuntime,
  /// The contract handler.
  /// `handle(state_ptr, state_len, action_ptr, action_len) -> result_ptr`
  /// Length of the result can be obtained by calling `WasmRuntime::result_len`.
  handle: v8::Global<v8::Function>,
  /// The length of the updated state.
  /// `get_len() -> usize`
  result_len: v8::Global<v8::Function>,
  /// Memory allocator for the contract.
  /// `_alloc(size) -> ptr`
  allocator: v8::Global<v8::Function>,
  /// `WebAssembly.Instance.exports` object.
  exports: v8::Global<v8::Object>,
  /// Allocated Smartweave ContractInfo
  sw_contract: (
    Vec<u8>,
    // Pointer to the allocated contract info
    u32,
    // Length of the allocated contract info
    usize,
  ),
}

impl WasmRuntime {
  pub async fn new(
    wasm: &[u8],
    contract: ContractInfo,
  ) -> Result<WasmRuntime, AnyError> {
    let mut rt = JsRuntime::new(Default::default());
    let contract = deno_core::serde_json::to_vec(&contract)?;
    // Get hold of the WebAssembly object.
    let wasm_obj = rt.execute_script("<anon>", "WebAssembly").unwrap();
    let (exports, handle, allocator, result_len, contract_ptr) = {
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

      let ns = v8::Object::new(scope);

      let read_state_str =
        v8::String::new(scope, "smartweave_read_state").unwrap();
      let read_state = |scope: &mut v8::HandleScope,
                        args: v8::FunctionCallbackArguments,
                        mut rv: v8::ReturnValue| {
        let ctx = scope.get_current_context();
        let global = ctx.global(scope);
        let exports_str = v8::String::new(scope, "exports").unwrap();
        let exports = global.get(scope, exports_str.into()).unwrap();
        let exports = v8::Local::<v8::Object>::try_from(exports).unwrap();

        let mem_str = v8::String::new(scope, "memory").unwrap();
        let mem_obj = exports.get(scope, mem_str.into()).unwrap();
        let mem_obj = v8::Local::<v8::Object>::try_from(mem_obj).unwrap();

        let buffer_str = v8::String::new(scope, "buffer").unwrap();
        let buffer_obj = mem_obj.get(scope, buffer_str.into()).unwrap();

        let alloc_str = v8::String::new(scope, "_alloc").unwrap();
        let alloc_obj = exports.get(scope, alloc_str.into()).unwrap();
        let alloc = v8::Local::<v8::Function>::try_from(alloc_obj).unwrap();
        let undefined = v8::undefined(scope);

        let mem_buf =
          v8::Local::<v8::ArrayBuffer>::try_from(buffer_obj).unwrap();

        let store = mem_buf.get_backing_store();

        let tx_id_ptr = args
          .get(0)
          .to_number(scope)
          .unwrap()
          .int32_value(scope)
          .unwrap();

        let tx_id_len = args
          .get(1)
          .to_number(scope)
          .unwrap()
          .int32_value(scope)
          .unwrap();

        let tx_bytes = unsafe {
          get_backing_store_slice_mut(
            &store,
            tx_id_ptr as usize,
            tx_id_len as usize,
          )
        };

        let length_ptr = args
          .get(2)
          .to_number(scope)
          .unwrap()
          .int32_value(scope)
          .unwrap();

        let mut len_bytes = unsafe {
          get_backing_store_slice_mut(&store, length_ptr as usize, 4)
        };

        let mut tx_id = String::from_utf8_lossy(tx_bytes).to_string();

        let state = smartweave::read_contract_state(tx_id);
        let state = deno_core::serde_json::to_vec(&state).unwrap();

        let mut state_len = (state.len() as u32).to_le_bytes();
        len_bytes.swap_with_slice(&mut state_len);

        let state_len = v8::Number::new(scope, state.len() as f64);
        let state_ptr = wasm_alloc!(scope, alloc, undefined, state_len);

        rv.set(state_ptr);
      };

      let read_state_callback = v8::Function::new(scope, read_state).unwrap();
      ns.set(scope, read_state_str.into(), read_state_callback.into());

      let consume_gas_str = v8::String::new(scope, "consumeGas").unwrap();

      let consume_gas = |scope: &mut v8::HandleScope,
                         args: v8::FunctionCallbackArguments,
                         _: v8::ReturnValue| {
        let inc = args
          .get(0)
          .to_number(scope)
          .unwrap()
          .int32_value(scope)
          .unwrap();

        let ctx = scope.get_current_context();
        let global = ctx.global(scope);
        let cost_str = v8::String::new(scope, "COST").unwrap();
        let cost = global.get(scope, cost_str.into()).unwrap();
        let cost = cost.int32_value(scope).unwrap();
        let cost = cost + inc;

        let cost = v8::Number::new(scope, cost as f64);
        global.set(scope, cost_str.into(), cost.into()).unwrap();
      };

      let consume_gas_callback = v8::Function::new(scope, consume_gas).unwrap();
      ns.set(scope, consume_gas_str.into(), consume_gas_callback.into());

      let ns_str = v8::String::new(scope, "3em").unwrap();
      imports.set(scope, ns_str.into(), ns.into());

      // wasi_snapshot_preview1
      let wasi_ns = v8::Object::new(scope);
      let wasi_snapshot_preview1_str =
        v8::String::new(scope, "wasi_snapshot_preview1").unwrap();

      let wasi_fd_close = v8::String::new(scope, "fd_close").unwrap();
      let wasi_fd_close_callback =
        |_: &mut v8::HandleScope,
         _: v8::FunctionCallbackArguments,
         _: v8::ReturnValue| {
          // No-op.
        };
      let wasi_fd_close_callback =
        v8::Function::new(scope, wasi_fd_close_callback).unwrap();
      wasi_ns.set(scope, wasi_fd_close.into(), wasi_fd_close_callback.into());

      let wasi_fd_seek = v8::String::new(scope, "fd_seek").unwrap();
      let wasi_fd_seek_callback =
        |_: &mut v8::HandleScope,
         _: v8::FunctionCallbackArguments,
         _: v8::ReturnValue| {
          // No-op.
        };
      let wasi_fd_seek_callback =
        v8::Function::new(scope, wasi_fd_seek_callback).unwrap();
      wasi_ns.set(scope, wasi_fd_seek.into(), wasi_fd_seek_callback.into());

      let wasi_fd_write = v8::String::new(scope, "fd_write").unwrap();
      let wasi_fd_write_callback =
        |_: &mut v8::HandleScope,
         _: v8::FunctionCallbackArguments,
         _: v8::ReturnValue| {
          // No-op.
        };
      let wasi_fd_write_callback =
        v8::Function::new(scope, wasi_fd_write_callback).unwrap();
      wasi_ns.set(scope, wasi_fd_write.into(), wasi_fd_write_callback.into());

      imports.set(scope, wasi_snapshot_preview1_str.into(), wasi_ns.into());

      let instance = instance_constructor
        .new_instance(scope, &[module.into(), imports.into()])
        .unwrap();

      let exports_str = v8::String::new(scope, "exports").unwrap();
      let exports = instance.get(scope, exports_str.into()).unwrap();
      let exports = v8::Local::<v8::Object>::try_from(exports)?;

      let ctx = scope.get_current_context();
      let global = ctx.global(scope);
      global
        .set(scope, exports_str.into(), exports.into())
        .unwrap();

      let cost_str = v8::String::new(scope, "COST").unwrap();
      let cost = v8::Number::new(scope, 0.0);
      global.set(scope, cost_str.into(), cost.into()).unwrap();

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

      let undefined = v8::undefined(scope);
      let alloc_obj = allocator.get(scope).to_object(scope).unwrap();
      let alloc = v8::Local::<v8::Function>::try_from(alloc_obj)?;

      let contact_info_len = v8::Number::new(scope, contract.len() as f64);
      let contract_ptr = wasm_alloc!(scope, alloc, undefined, contact_info_len);
      let contract_ptr_u32 = contract_ptr.uint32_value(scope).unwrap();

      (exports, handle, allocator, result_len, contract_ptr_u32)
    };

    let len = contract.len();
    Ok(Self {
      rt,
      handle,
      allocator,
      result_len,
      exports,
      sw_contract: (contract, contract_ptr, len),
    })
  }

  pub fn get_cost(&mut self) -> usize {
    let scope = &mut self.rt.handle_scope();
    let ctx = scope.get_current_context();
    let global = ctx.global(scope);
    let cost_str = v8::String::new(scope, "COST").unwrap();
    let cost = global.get(scope, cost_str.into()).unwrap();
    let cost = v8::Local::<v8::Number>::try_from(cost).unwrap();
    let cost = cost.int32_value(scope).unwrap();
    cost as usize
  }

  pub async fn call(
    &mut self,
    state: &mut [u8],
    action: &mut [u8],
  ) -> Result<Vec<u8>, AnyError> {
    let result = {
      let scope = &mut self.rt.handle_scope();
      let undefined = v8::undefined(scope);

      let alloc_obj = self.allocator.get(scope).to_object(scope).unwrap();
      let alloc = v8::Local::<v8::Function>::try_from(alloc_obj)?;

      let state_len = v8::Number::new(scope, state.len() as f64);
      let action_len = v8::Number::new(scope, action.len() as f64);

      let contract_ptr = v8::Number::new(scope, self.sw_contract.1 as f64);
      let contract_info_len = v8::Number::new(scope, self.sw_contract.2 as f64);

      // Offset in memory for start of the block.
      let local_ptr = wasm_alloc!(scope, alloc, undefined, state_len);
      let local_ptr_u32 = local_ptr.uint32_value(scope).unwrap();
      let action_ptr = wasm_alloc!(scope, alloc, undefined, action_len);
      let action_ptr_u32 = action_ptr.uint32_value(scope).unwrap();

      let exports_obj = self.exports.get(scope).to_object(scope).unwrap();

      let mem_str = v8::String::new(scope, "memory").unwrap();
      let mem_obj = exports_obj.get(scope, mem_str.into()).unwrap();
      let mem_obj = v8::Local::<v8::Object>::try_from(mem_obj)?;

      let buffer_str = v8::String::new(scope, "buffer").unwrap();
      let buffer_obj = mem_obj.get(scope, buffer_str.into()).unwrap();

      let mem_buf = v8::Local::<v8::ArrayBuffer>::try_from(buffer_obj)?;

      // O HOLY Backin' store.
      let store = mem_buf.get_backing_store();

      let mut state_mem_region = unsafe {
        get_backing_store_slice_mut(&store, local_ptr_u32 as usize, state.len())
      };

      state_mem_region.swap_with_slice(state);

      let action_mem_region = unsafe {
        get_backing_store_slice_mut(
          &store,
          action_ptr_u32 as usize,
          action.len(),
        )
      };

      action_mem_region.swap_with_slice(state);

      let contract_mem_region = unsafe {
        get_backing_store_slice_mut(
          &store,
          self.sw_contract.1 as usize,
          self.sw_contract.2,
        )
      };

      contract_mem_region.swap_with_slice(&mut self.sw_contract.0);

      let mut action_region = unsafe {
        get_backing_store_slice_mut(
          &store,
          action_ptr_u32 as usize,
          action.len(),
        )
      };

      action_region.swap_with_slice(action);

      let handler_obj = self.handle.get(scope).to_object(scope).unwrap();
      let handle = v8::Local::<v8::Function>::try_from(handler_obj)?;

      let result_ptr = handle
        .call(
          scope,
          undefined.into(),
          &[
            local_ptr,
            state_len.into(),
            action_ptr,
            action_ptr.into(),
            contract_ptr.into(),
            contract_info_len.into(),
          ],
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
  use crate::runtime::wasm::WasmRuntime;
  use deno_core::serde_json::json;
  use deno_core::serde_json::Value;
  use std::sync::atomic::Ordering;

  #[tokio::test]
  async fn test_wasm_runtime_contract() {
    let mut rt = WasmRuntime::new(
      include_bytes!("./testdata/01_wasm/01_wasm.wasm"),
      Default::default(),
    )
    .await
    .unwrap();

    let mut prev_state = json!({
      "counter": 0,
    });

    let mut prev_state_bytes =
      deno_core::serde_json::to_vec(&prev_state).unwrap();
    let state = rt.call(&mut prev_state_bytes, &mut []).await.unwrap();

    let state: Value = deno_core::serde_json::from_slice(&state).unwrap();

    assert_eq!(state.get("counter").unwrap(), 1);

    // No cost without metering.
    assert_eq!(rt.get_cost(), 0);
  }

  #[tokio::test]
  async fn test_wasm_runtime_asc() {
    let mut rt = WasmRuntime::new(
      include_bytes!("./testdata/02_wasm/02_wasm.wasm"),
      Default::default(),
    )
    .await
    .unwrap();

    let mut prev_state = json!({
      "counter": 0,
    });

    for i in 1..100 {
      let mut prev_state_bytes =
        deno_core::serde_json::to_vec(&prev_state).unwrap();
      let state = rt.call(&mut prev_state_bytes, &mut []).await.unwrap();

      let state: Value = deno_core::serde_json::from_slice(&state).unwrap();

      assert_eq!(state.get("counter").unwrap(), i);
      prev_state = state;
    }

    // No cost without metering.
    assert_eq!(rt.get_cost(), 0);
  }
}
