use napi::bindgen_prelude::*;
use napi::sys::{napi_env, napi_value};
use napi_derive::napi;
use serde_json::Value;
use std::collections::HashMap;
use std::panic;
use three_em_arweave::arweave::{Arweave, ManualLoadedContract};
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_arweave::gql_result::{GQLEdgeInterface, GQLTagInterface};
use three_em_arweave::miscellaneous::ContractType;
use three_em_executor::execute_contract as execute;
use three_em_executor::simulate_contract as simulate;
use three_em_executor::utils::create_simulated_transaction;
use three_em_executor::ExecuteResult;
use three_em_executor::ValidityTable;
use tokio::runtime::Handle;

#[cfg(target_os = "macos")]
#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[napi(object)]
pub struct ExecuteContractResult {
  pub state: serde_json::Value,
  pub result: serde_json::Value,
  pub validity: HashMap<String, serde_json::Value>,
  pub exm_context: serde_json::Value,
  pub updated: bool,
}

#[napi(object)]
pub struct ExecuteConfig {
  pub host: String,
  pub port: i32,
  pub protocol: String,
}

#[napi(object)]
pub struct Tag {
  pub name: String,
  pub value: String,
}

#[napi(object)]
pub struct Block {
  pub height: String,
  pub indep_hash: String,
  pub timestamp: String,
}

#[napi(object)]
pub struct SimulateInput {
  pub id: String,
  pub owner: String,
  pub quantity: String,
  pub reward: String,
  pub target: Option<String>,
  pub tags: Vec<Tag>,
  pub block: Option<Block>,
  pub input: String,
}

#[napi(object)]
pub enum SimulateContractType {
  JAVASCRIPT,
  WASM,
}

#[napi(object)]
pub struct ContractSource {
  pub contract_src: Buffer,
  pub contract_type: SimulateContractType,
}

#[napi(object)]
pub struct SimulateExecutionContext {
  pub contract_id: String,
  pub maybe_contract_source: Option<ContractSource>,
  pub interactions: Vec<SimulateInput>,
  pub contract_init_state: Option<String>,
  pub maybe_config: Option<ExecuteConfig>,
  pub maybe_cache: Option<bool>,
  pub maybe_bundled_contract: Option<bool>,
  pub maybe_settings: Option<HashMap<String, serde_json::Value>>,
  pub maybe_exm_context: Option<serde_json::Value>,
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

fn get_gateway(
  maybe_config: Option<ExecuteConfig>,
  use_cache: Option<bool>,
) -> Arweave {
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

  let use_cache_bool = use_cache.unwrap_or(true);
  if use_cache_bool {
    Arweave::new(
      arweave_port,
      arweave_gateway,
      arweave_protocol,
      ArweaveCache::new(),
    )
  } else {
    Arweave::new_no_cache(arweave_port, arweave_gateway, arweave_protocol)
  }
}

fn get_result(
  process_result: std::result::Result<
    ExecuteResult,
    three_em_arweave::miscellaneous::CommonError,
  >,
) -> Option<ExecuteContractResult> {
  if process_result.is_ok() {
    match process_result.unwrap() {
      ExecuteResult::V8(data) => {
        let (state, result, validity, exm_context) = (
          data.state,
          data.result.unwrap_or(Value::Null),
          data.validity,
          data.context,
        );
        Some(ExecuteContractResult {
          state,
          result,
          validity: validity_to_hashmap(validity),
          exm_context: serde_json::to_value(exm_context).unwrap(),
          updated: data.updated,
        })
      }
      ExecuteResult::Evm(..) => todo!(),
    }
  } else {
    process_result.unwrap();
    None
  }
}

#[napi]
async fn simulate_contract(
  context: SimulateExecutionContext,
) -> Result<ExecuteContractResult> {
  let SimulateExecutionContext {
    contract_id,
    interactions,
    contract_init_state,
    maybe_config,
    maybe_cache,
    maybe_bundled_contract,
    maybe_settings,
    maybe_exm_context,
    maybe_contract_source,
  } = context;

  let result = tokio::task::spawn_blocking(move || {
    panic::catch_unwind(|| {
      Handle::current().block_on(async move {
        let arweave = get_gateway(maybe_config, maybe_cache.clone());

        let real_interactions: Vec<GQLEdgeInterface> = interactions
          .into_iter()
          .map(|data| {
            let tags: Vec<GQLTagInterface> = data
              .tags
              .into_iter()
              .map(|tag| GQLTagInterface {
                name: tag.name.to_string(),
                value: tag.value.to_string(),
              })
              .collect::<Vec<GQLTagInterface>>();

            let (height, timestamp, indep_hash) =
              if let Some(block_data) = data.block {
                (
                  Some(block_data.height),
                  Some(block_data.timestamp),
                  Some(block_data.indep_hash),
                )
              } else {
                (None, None, None)
              };

            let transaction = create_simulated_transaction(
              data.id,
              data.owner,
              data.quantity,
              data.reward,
              data.target,
              tags,
              height,
              indep_hash,
              timestamp,
              data.input,
            );

            transaction
          })
          .collect();

        let manual_loaded_contract = {
          if let Some(contract_source) = maybe_contract_source {
            let loaded_contract = ManualLoadedContract {
              contract_src: contract_source.contract_src.into(),
              contract_type: match contract_source.contract_type {
                SimulateContractType::JAVASCRIPT => ContractType::JAVASCRIPT,
                SimulateContractType::WASM => ContractType::WASM,
              },
            };
            Some(loaded_contract)
          } else {
            None
          }
        };

        let result = simulate(
          contract_id,
          contract_init_state,
          real_interactions,
          &arweave,
          maybe_cache,
          maybe_bundled_contract,
          maybe_settings,
          maybe_exm_context,
          manual_loaded_contract,
        )
        .await;

        get_result(result)
      })
    })
  })
  .await;

  if let Ok(catcher) = result {
    if let Ok(processing) = catcher {
      if let Some(result) = processing {
        return Ok(result);
      }
    }
  }

  return Err(Error::new(
    Status::Unknown,
    "Contract could not be processed".to_string(),
  ));
}

#[napi]
async fn execute_contract(
  tx: String,
  maybe_height: Option<u32>,
  maybe_config: Option<ExecuteConfig>,
) -> Result<ExecuteContractResult> {
  let result = tokio::task::spawn_blocking(move || {
    Handle::current().block_on(async move {
      let arweave = get_gateway(maybe_config, None);

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
  use crate::{
    execute_contract, get_gateway, simulate_contract, ContractSource,
    ExecuteConfig, SimulateContractType, SimulateExecutionContext,
    SimulateInput,
  };
  use three_em_arweave::arweave::get_cache;

  // #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
  // #[should_panic]
  // pub async fn no_cache_test() {
  //   get_gateway(None, Some(false));
  //   get_cache();
  // }

  #[tokio::test]
  pub async fn with_cache_test() {
    get_gateway(None, None);
    get_cache();
  }

  // #[tokio::test]
  // pub async fn test_execute_contract() {
  //   let contract = execute_contract(
  //     String::from("yAovBvlYWiIBx6i7hPSo2f5hNJpG6Wdq4eDyiudm1_M"),
  //     None,
  //     Some(ExecuteConfig {
  //       host: String::from("www.arweave.run"),
  //       port: 443,
  //       protocol: String::from("https"),
  //     }),
  //   )
  //   .await;
  //   let contract_result = contract.unwrap().state;
  //   println!("{}", contract_result);
  //   assert_eq!(
  //     contract_result.get("name").unwrap().as_str().unwrap(),
  //     "VERTO"
  //   );
  // }

  #[tokio::test]
  pub async fn simulate_contract_test() {
    let execution_context: SimulateExecutionContext =
      SimulateExecutionContext {
        contract_id: "KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ".into(),
        interactions: vec![SimulateInput {
          id: String::from("abcd"),
          owner: String::from("210392sdaspd-asdm-asd_sa0d1293-lc"),
          quantity: String::from("12301"),
          reward: String::from("12931293"),
          target: None,
          tags: vec![],
          block: None,
          input: serde_json::json!({}).to_string(),
        }],
        contract_init_state: Some(r#"{"counter": 2481}"#.into()),
        maybe_config: None,
        maybe_cache: Some(false),
        maybe_bundled_contract: None,
        maybe_settings: None,
        maybe_exm_context: None,
        maybe_contract_source: None,
      };

    let contract = simulate_contract(execution_context).await.unwrap();

    let contract_result = contract.state;
    println!("{}", contract_result);
    assert_eq!(contract_result.get("counter").unwrap(), 2482);
  }

  #[tokio::test]
  pub async fn simulate_contract_test_bundled() {
    let execution_context: SimulateExecutionContext =
      SimulateExecutionContext {
        contract_id: "RadpzdYtVrQiS25JR1hGxZppwCXVCel_nfXk-noyFmc".into(),
        interactions: vec![
          SimulateInput {
            id: String::from("abcd"),
            owner: String::from("210392sdaspd-asdm-asd_sa0d1293-lc"),
            quantity: String::from("12301"),
            reward: String::from("12931293"),
            target: None,
            tags: vec![],
            block: None,
            input: serde_json::json!({}).to_string(),
          },
          SimulateInput {
            id: String::from("abcd"),
            owner: String::from("210392sdaspd-asdm-asd_sa0d1293-lc"),
            quantity: String::from("12301"),
            reward: String::from("12931293"),
            target: None,
            tags: vec![],
            block: None,
            input: serde_json::json!({}).to_string(),
          },
        ],
        contract_init_state: Some(r#"2"#.into()),
        maybe_config: None,
        maybe_cache: Some(false),
        maybe_bundled_contract: Some(true),
        maybe_settings: None,
        maybe_exm_context: None,
        maybe_contract_source: None,
      };

    let contract = simulate_contract(execution_context).await.unwrap();

    let contract_result = contract.state;
    println!("{}", contract_result);
    assert_eq!(contract_result, 4);
  }

  #[tokio::test]
  pub async fn simulate_contract_test_custom_source() {
    let contract_source_bytes =
      include_bytes!("../../../testdata/contracts/user-registry2.js");
    let contract_source_vec = contract_source_bytes.to_vec();
    let execution_context: SimulateExecutionContext =
      SimulateExecutionContext {
        contract_id: String::new(),
        interactions: vec![SimulateInput {
          id: String::from("abcd"),
          owner: String::from("210392sdaspd-asdm-asd_sa0d1293-lc"),
          quantity: String::from("12301"),
          reward: String::from("12931293"),
          target: None,
          tags: vec![],
          block: None,
          input: serde_json::json!({
            "username": "Andres"
          })
          .to_string(),
        }],
        contract_init_state: Some(r#"{"users": []}"#.into()),
        maybe_config: None,
        maybe_cache: Some(false),
        maybe_bundled_contract: None,
        maybe_settings: None,
        maybe_exm_context: None,
        maybe_contract_source: Some(ContractSource {
          contract_src: contract_source_vec.into(),
          contract_type: SimulateContractType::JAVASCRIPT,
        }),
      };

    let contract = simulate_contract(execution_context).await.unwrap();

    let contract_result = contract.state;
    println!("{}", contract_result);
    assert_eq!(
      contract_result.get("users").unwrap(),
      &serde_json::json!([{"username": "Andres"}])
    );
    assert_eq!(contract.result.as_str().unwrap(), "Hello World");
    assert_eq!(contract.updated, true);
  }
}
