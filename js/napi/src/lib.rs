use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_executor::execute_contract as execute;
use three_em_executor::ExecuteResult;
use three_em_executor::ValidityTable;
use tokio::runtime::Handle;

#[cfg(target_os = "macos")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[napi(object)]
pub struct ExecuteContractResult {
  pub state: serde_json::Value,
  pub validity: HashMap<String, serde_json::Value>,
}

#[napi(object)]
pub struct ExecuteConfig {
  pub host: String,
  pub port: i32,
  pub protocol: String
}

// Convert the ValidityTable from an IndexMap to HashMap
#[inline]
fn validity_to_hashmap(
  table: ValidityTable,
) -> HashMap<String, serde_json::Value> {
  let mut map = HashMap::new();
  for (k, v) in table {
    map.insert(k, v);
  }
  map
}

#[napi]
async fn execute_contract(
  tx: String,
  maybe_height: Option<u32>,
  maybe_config: Option<ExecuteConfig>
) -> Result<ExecuteContractResult> {

  let arweave_gateway = maybe_config.as_ref().map(|item| item.host.to_owned()).unwrap_or("arweave.net".to_string());
  let arweave_protocol = maybe_config.as_ref().map(|item| item.protocol.to_owned()).unwrap_or("https".to_string());
  let arweave_port = maybe_config.as_ref().map(|item| item.port).unwrap_or(443 as i32);

  let result = tokio::task::spawn_blocking(move || {
    Handle::current().block_on(async move {
      let arweave = Arweave::new(
        arweave_port,
        arweave_gateway,
        arweave_protocol,
        ArweaveCache::new(),
      );
      let result = execute(
        &arweave,
        tx,
        None,
        None,
        maybe_height.map(|h| h as usize),
        true,
        false,
      )
      .await;

      if result.is_ok() {
        match result.unwrap() {
          ExecuteResult::V8(state, validity) => Some(ExecuteContractResult {
            state,
            validity: validity_to_hashmap(validity),
          }),
          ExecuteResult::Evm(..) => todo!(),
        }
      } else {
        result.unwrap();
        None
      }
    })
  })
  .await
      .unwrap();

  if let Some(result) = result {
    Ok(result)
  } else {
    Err(Error::new(Status::Unknown, "Contract could not be processed".to_string()))
  }
}

#[cfg(test)]
mod tests {
  use crate::{execute_contract, ExecuteConfig};

  #[tokio::test]
  pub async fn test_execute_contract() {
    let contract = execute_contract(String::from("xRkYokQfFHLh2K9slmghlXNptKrqQdDZoy75JGsv89M"), None, Some(ExecuteConfig {
      host: String::from("www.arweave.run"),
      port: 443,
      protocol: String::from("https")
    })).await;
    let contract_result = contract.unwrap().state;
    println!("{}", contract_result);
    assert_eq!(contract_result.get("name").unwrap().as_str().unwrap(), "VERTO #0");
  }

}
