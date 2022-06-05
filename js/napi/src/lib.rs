use napi::bindgen_prelude::*;
use napi::sys::{napi_env, napi_value};
use napi_derive::napi;
use serde_json::Value;
use std::collections::HashMap;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_arweave::gql_result::GQLEdgeInterface;
use three_em_executor::execute_contract as execute;
use three_em_executor::simulate_contract as simulate;
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
  pub protocol: String,
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

fn get_gateway(maybe_config: Option<ExecuteConfig>) -> Arweave {
  let arweave_gateway = maybe_config
    .as_ref()
    .map(|item| item.host.to_owned())
    .unwrap_or("arweave.net".to_string());
  let arweave_protocol = maybe_config
    .as_ref()
    .map(|item| item.protocol.to_owned())
    .unwrap_or("https".to_string());
  let arweave_port = maybe_config
    .as_ref()
    .map(|item| item.port)
    .unwrap_or(443 as i32);

  Arweave::new(
    arweave_port,
    arweave_gateway,
    arweave_protocol,
    ArweaveCache::new(),
  )
}

fn get_result(
  process_result: std::result::Result<
    ExecuteResult,
    three_em_arweave::miscellaneous::CommonError,
  >,
) -> Option<ExecuteContractResult> {
  if process_result.is_ok() {
    match process_result.unwrap() {
      ExecuteResult::V8(state, validity) => Some(ExecuteContractResult {
        state,
        validity: validity_to_hashmap(validity),
      }),
      ExecuteResult::Evm(..) => todo!(),
    }
  } else {
    process_result.unwrap();
    None
  }
}

/// TODO: Type `interactions` to GQLEdgeInterface (figure out how to make a napi object for it without re-coding existent struct)

#[napi]
async fn simulate_contract(
  contract_id: String,
  contract_init_state: Option<String>,
  interactions: Value,
  maybe_config: Option<ExecuteConfig>,
) -> Result<ExecuteContractResult> {
  let result = tokio::task::spawn_blocking(move || {
    Handle::current().block_on(async move {
      let arweave = get_gateway(maybe_config);

      let real_interactions: Vec<GQLEdgeInterface> =
        serde_json::from_value(interactions).unwrap();

      let result = simulate(
        contract_id,
        contract_init_state,
        real_interactions,
        &arweave,
      )
      .await;

      get_result(result)
    })
  })
  .await
  .unwrap();

  if let Some(result) = result {
    Ok(result)
  } else {
    Err(Error::new(
      Status::Unknown,
      "Contract could not be processed".to_string(),
    ))
  }
}

#[napi]
async fn execute_contract(
  tx: String,
  maybe_height: Option<u32>,
  maybe_config: Option<ExecuteConfig>,
) -> Result<ExecuteContractResult> {
  let result = tokio::task::spawn_blocking(move || {
    Handle::current().block_on(async move {
      let arweave = get_gateway(maybe_config);

      let result = execute(
        tx,
        maybe_height.map(|h| h as usize),
        true,
        false,
        None,
        None,
        &arweave,
      )
      .await;

      get_result(result)
    })
  })
  .await
  .unwrap();

  if let Some(result) = result {
    Ok(result)
  } else {
    Err(Error::new(
      Status::Unknown,
      "Contract could not be processed".to_string(),
    ))
  }
}

#[cfg(test)]
mod tests {
  use crate::{execute_contract, simulate_contract, ExecuteConfig};

  #[tokio::test]
  pub async fn test_execute_contract() {
    let contract = execute_contract(
      String::from("xRkYokQfFHLh2K9slmghlXNptKrqQdDZoy75JGsv89M"),
      None,
      Some(ExecuteConfig {
        host: String::from("www.arweave.run"),
        port: 443,
        protocol: String::from("https"),
      }),
    )
    .await;
    let contract_result = contract.unwrap().state;
    println!("{}", contract_result);
    assert_eq!(
      contract_result.get("name").unwrap().as_str().unwrap(),
      "VERTO #0"
    );
  }

  #[tokio::test]
  pub async fn simulate_contract_test() {
    let contract = simulate_contract(
      "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ".into(),
      Some(r#"{"counter": 2481}"#.into()),
      serde_json::json!([{
        "cursor": "",
        "node": {
          "id": "abcd",
          "owner": {
            "address": "paykB5SUNHFBIL3V6HLfUIL9n_X3Anp1JSf7bXMsTRQ"
          },
          "tags": [{
            "name": "Input",
            "value": "{}"
          }],
          "block": {
            "id": "kaskdlasfl",
            "timestamp": 1239019230,
            "height": 1
          }
        }
      }]),
      None,
    )
    .await
    .unwrap();

    let contract_result = contract.state;
    println!("{}", contract_result);
    assert_eq!(contract_result.get("counter").unwrap(), 2482);
  }
}
