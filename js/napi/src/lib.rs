use napi::bindgen_prelude::*;
use napi_derive::napi;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_executor::execute_contract as execute;
use three_em_executor::ExecuteResult;
use tokio::runtime::Handle;

#[cfg(target_os = "macos")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[napi(object)]
pub struct ExecuteContractResult {
  pub state: serde_json::Value,
}

#[napi]
async fn execute_contract(
  tx: String,
  maybe_height: Option<u32>,
) -> Result<ExecuteContractResult> {
  let result = tokio::task::spawn_blocking(move || {
    Handle::current().block_on(async move {
      let arweave = Arweave::new(
        443,
        "arweave.net".to_string(),
        String::from("https"),
        ArweaveCache::new(),
      );
      let result = execute(
        &arweave,
        tx,
        None,
        None,
        maybe_height.map(|h| h as usize),
        true,
        false
      )
      .await
      .unwrap();

      match result {
        ExecuteResult::V8(state, ..) => ExecuteContractResult { state },
        ExecuteResult::Evm(..) => todo!(),
      }
    })
  })
  .await
  .unwrap();
  Ok(result)
}
