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
