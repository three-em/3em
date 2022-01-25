use proc_macro::TokenStream;
use syn::ItemFn;
use quote::quote;

#[proc_macro_attribute]
pub fn handler(_attr: TokenStream, input: TokenStream) -> TokenStream {
  let func = syn::parse::<ItemFn>(input).unwrap();

  let fn_inputs = &func.sig.inputs;
  let fn_output = &func.sig.output;
  let fn_generics = &func.sig.generics;
  let fn_block = &func.block;

  TokenStream::from(quote! {
    use ::std::alloc::alloc;
    use ::std::alloc::dealloc;
    use ::std::alloc::Layout;
    use ::std::mem::align_of;
    use ::std::panic;
    use ::std::sync::Once;  
    
    #[link(wasm_import_module = "3em")]
    extern "C" {
      fn smartweave_read_state(
        // `ptr` is the pointer to the base64 URL encoded sha256 txid.
        ptr: *const u8,
        ptr_len: usize,
        // Pointer to the 4 byte array to store the length of the state.
        result_len_ptr: *mut u8,
      ) -> *mut u8;
      fn throw_error(ptr: *const u8, len: usize);
    }
    
    #[no_mangle]
    pub unsafe fn _alloc(len: usize) -> *mut u8 {
      let align = align_of::<usize>();
      let layout = Layout::from_size_align_unchecked(len, align);
      alloc(layout)
    }

    #[no_mangle]
    pub unsafe fn _dealloc(ptr: *mut u8, size: usize) {
      let align = align_of::<usize>();
      let layout = Layout::from_size_align_unchecked(size, align);
      dealloc(ptr, layout);
    }

    #[no_mangle]
    pub fn panic_hook(info: &panic::PanicInfo) {
      let payload = info.payload();
      let payload_str = match payload.downcast_ref::<&str>() {
        Some(s) => s,
        None => match payload.downcast_ref::<String>() {
          Some(s) => s,
          None => "Box<Any>",
        },
      };
      let msg = format!("{}", payload_str);
      let msg_ptr = msg.as_ptr();
      let msg_len = msg.len();
      unsafe {
        throw_error(msg_ptr, msg_len);
      }
      std::mem::forget(msg);
    }

    static mut LEN: usize = 0;

    #[no_mangle]
    pub extern "C" fn get_len() -> usize {
      unsafe { LEN }
    }

    #[no_mangle]
    pub extern "C" fn handle(
      state: *mut u8,
      state_size: usize,
      action: *mut u8,
      action_size: usize,
      contract_info: *mut u8,
      contract_info_size: usize,
    ) -> *const u8 {
      static SET_HOOK: Once = Once::new();
      SET_HOOK.call_once(|| {
        panic::set_hook(Box::new(panic_hook));
      });

      let state_buf = unsafe { Vec::from_raw_parts(state, state_size, state_size) };
      let action_buf = unsafe { Vec::from_raw_parts(action, action_size, action_size) };

      fn __inner_handler #fn_generics (#fn_inputs) #fn_output #fn_block
      let output_state = __inner_handler(
        serde_json::from_slice(&state_buf).unwrap(),
        serde_json::from_slice(&action_buf).unwrap(),
      );

      let output_buf = serde_json::to_vec(&output_state).unwrap();
      let output = output_buf.as_slice().as_ptr();

      unsafe {
        LEN = output_buf.len();
      }

      ::std::mem::forget(state_buf);

      output
    }
  })
}
