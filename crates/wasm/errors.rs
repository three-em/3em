use deno_core::error::AnyError;
use deno_core::Extension;
use deno_core::OpState;
use deno_core::op_sync;
use deno_core::include_js_files;

pub fn init() -> Extension {
  Extension::builder().js(
    include_js_files!(
      prefix "3em:wasm/errors",
      "errors.js",
    )
  ).ops(vec![
    ("wasm_errors_error_message", op_sync(error_message)),
  ]).build()
}

/// Non recoverable / `panic!` like error.
pub struct WasmError(pub String);

pub fn error_message(
  state: &mut OpState,
  msg: String,
  _: (),
) -> Result<(), AnyError> {
  state.put(WasmError(msg));
  Ok(())
}

