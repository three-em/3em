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
      // Note: We do not snapshot smartweave as an extension.
    ],
    will_snapshot: true,
    ..Default::default()
  });

  let files = get_js_files("src/crates/smartweave");
  let display_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
  for file in files {
    println!("cargo:rerun-if-changed={}", file.display());
    let display_path = file.strip_prefix(display_root).unwrap();
    let display_path_str = display_path.display().to_string();
    snapshot_runtime
      .execute_script(
        &("3em:".to_string() + &display_path_str.replace('\\', "/")),
        &std::fs::read_to_string(&file).unwrap(),
      )
      .unwrap();
  }

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

fn get_js_files(d: &str) -> Vec<PathBuf> {
  let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
  let mut js_files = std::fs::read_dir(d)
    .unwrap()
    .map(|dir_entry| {
      let file = dir_entry.unwrap();
      manifest_dir.join(file.path())
    })
    .filter(|path| path.extension().unwrap_or_default() == "js")
    .collect::<Vec<PathBuf>>();
  js_files.sort();
  js_files
}
