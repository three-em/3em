#[path = "smartweave.rs"]
mod smartweave;
use deno_core::Extension;
use deno_web::BlobStore;

pub fn get_extensions() -> Vec<Extension> {
  vec![
    deno_webidl::init(),
    deno_url::init(),
    deno_web::init(BlobStore::default(), None),
    deno_crypto::init(None),
    smartweave::init(),
  ]
}
