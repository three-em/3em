use deno_core::JsRuntime;
use deno_core::RuntimeOptions;
use deno_web::BlobStore;
use std::env;
use std::path::Path;
use std::path::PathBuf;

// Adapted from deno_runtime
// https://github.com/denoland/deno/blob/fdf890a68d3d54d40c766fd78faeccb20bd2e2c6/runtime/build.rs#L37-L41
fn create_snapshot(snapshot_path: &Path) {
  let mut snapshot_runtime = JsRuntime::new(RuntimeOptions {
    extensions: vec![
      deno_webidl::init(),
      deno_url::init(),
      deno_web::init(BlobStore::default(), None),
      deno_crypto::init(None),
      three_em_smartweave::init(),
    ],
    will_snapshot: true,
    ..Default::default()
  });

  let snapshot = snapshot_runtime.snapshot();
  let snapshot_slice: &[u8] = &*snapshot;
  println!("Snapshot size: {}", snapshot_slice.len());
  std::fs::write(&snapshot_path, snapshot_slice).unwrap();
  println!("Snapshot written to: {} ", snapshot_path.display());
}

fn main() {
  let o = PathBuf::from(env::var_os("OUT_DIR").unwrap());
  let runtime_snapshot_path = o.join("CLI_SNAPSHOT.bin");

  create_snapshot(&runtime_snapshot_path);
}
