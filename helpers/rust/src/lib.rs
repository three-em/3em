use std::panic;
pub use three_em_macro::*;

pub mod alloc;

#[link(wasm_import_module = "3em")]
extern "C" {
  fn throw_error(ptr: *const u8, len: usize);
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
