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
  pub errors: HashMap<String, String>,
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
  pub maybe_exm_context: Option<String>,
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
        let (state, result, validity, exm_context, errors) = (
          data.state,
          data.result.unwrap_or(Value::Null),
          data.validity,
          data.context,
          data.errors,
        );
        Some(ExecuteContractResult {
          state,
          result,
          validity: validity_to_hashmap(validity),
          exm_context: serde_json::to_value(exm_context).unwrap(),
          updated: data.updated,
          errors,
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
  use std::collections::HashMap;
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
  pub async fn simulate_counter_failure() {
    let contract_source_bytes =
      include_bytes!("../../../testdata/contracts/counter_error.js");
    let contract_source_vec = contract_source_bytes.to_vec();
    let execution_context: SimulateExecutionContext =
      SimulateExecutionContext {
        contract_id: String::new(),
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
        contract_init_state: Some(r#"{"counts": 0}"#.into()),
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

    assert_eq!(contract.errors.len(), 1);
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

  #[tokio::test]
  pub async fn simulate_contract_ans() {
    let contract_source_bytes =
      include_bytes!("../../../testdata/contracts/ans.js");
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
                  input: serde_json::json!({"jwk_n":"iwLlbpT6b7rYOV6aJTTeSD5wXdqZYwYBGphdOclGeftR3wJcLp5OzWrQqexbfpvi-CBkxz0yroX4Of9Ikv_qoakIoz1qQsQztD2UR2MI6SAvbGPn8swKL7QId2OtyRtOJ3TAEuULt1lpA0nj67dOKNq-2HyhDyuCaZ6TTC2Luk5QUrqWKs9ix8xBNM_O20lmRGgDsrhNtTMca2tnkCk4JffGrEDVsF6iRM4Ls_KOcrAJlsrpSypi2M7O4rQkcyWaBerXomtkj4I5KBXnr5J9_sdLS-2esAHzpArNA2gnYi8aqtlJgSYWAX9TTItqjszK6kIIBlPJCMn7K1a4p2kJilJpOsPY9VFaPP5B5_ie2V87_6xpqgmMMjBzc2zS9MBkg55JP-sImXBMJQAvL89muSYIDAdqh8Z6dIEiwIGnnWHCbZKsPjkDV3Cmx1br4zx73Kp6fYD7lTNuFriKcRo7z7rcfdE5qJ-pamSpXZhEo1q4J08ZihOuA1LrvDuLrTwVMam_r0E5WJKHg28w2LD7pMgSUb7vhbMLTNSLMcvC19Tip2bBosfmX79tPoW0UQ5lUx1IykdIuARp6LTV5g8nESnxoAnRiBEQgJffXuMhnDbLu6lW_aoYRWM_uwGEu2So4a584mCc0ziNKb9ZfvmLxWS4M1NJd6Lnt9hHTkBxq-8","sig":"GZ0vreib6rUv/rG488ZdzFtIFiLWes2gptzRlX1p4fBebORurdShCPtQWvgXn3J9wTncnveuDLO2nK57gcliwqXYxSetWoJnyn5y4KeKWU3+zA+QUKoMntOu66XY3SF09taUMAfpDi73wOtBbN2vo+SR3NVjsxx3ibit2zannAOOf49CZABH6B2EujaVklv1pczfAzrVQPVU1z+XpGb7O1ydv380vc/gWT3yBduIjZLCvD3d8BK+6x3kLji8NsnqfFDTPCSVR11mZwedUGEVvG1ONYmxt7y8a5RZLWbdI2GeUroeOuimsUBqzPVORZ0ZH9vzpQ1lbHORYEvbpmq0wVn8w+kA5s9Z03S15y86ZX1260PangBLCOTUi8gZneKdByUkp18rl37XeH2CdBlkRrANdJZH/X3g0WUOkYEqSaVkw9zXO+a/sUmoDVGW6cqmdxN0ltJpLNd98nuDCHbS0FIIa9ksNwsQlnK5V/tZP+9Skw/lCBip6R8HKoRZhLuAsmh6k0eOKUFXJ7Objf40/+GvUGyNDJRxwtIvzQkTdALKNRDKNhhS4Kk8RH0ZhUIOhQHufg3HNaO3HmZeIOuo4pIOe1rma6oE4kiB8o7Je59I05d9PYIBgx619qMIWrRnc9z3sm/oPZvTNeLEL1G+46UVLe5MPkYpcXuQBzNe8ps=","txid":"0x7d07008ae820b889ad406142e5043dfd8d9ba6d9723fbef78a4c69ed294a65eb","mint_domain":"wearemintingyes", "function": "mint"})
                          .to_string(),
                }],
              contract_init_state: Some(String::from(include_str!("../../../testdata/contracts/ans.json"))),
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
    let str_state = contract_result.to_string();
    assert!(str_state.contains("wearemintingyes"));
  }

  #[tokio::test]
  pub async fn simulate_contract_ark() {
    let contract_source_bytes =
      include_bytes!("../../../testdata/contracts/ark.js");
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
                  "function": "createContainer",
                  "caller_address": "0x197f818c1313dc58b32d88078ecdfb40ea822614",
                  "type": "evm",
                  "label": "test-evm",
                  "sig": "0x0e8cda3652185efcb9e68cbd932836c46df14dcf3b052931f463f0cd189a0fdd619206accafbb44afc155ce1c629da2eda4370fd71376ad250f28b50711b112b1c"
                  }).to_string(),
          }],
          contract_init_state: Some(String::from(include_str!("../../../testdata/contracts/ark.json"))),
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
    assert_eq!(contract.errors.len(), 0);
    let contract_result = contract.state;
    let str_state = contract_result.to_string();
  }

  #[tokio::test]
  pub async fn simulate_deterministic_fetch_lazy() {
    let mut sets: HashMap<String, serde_json::Value> = HashMap::new();
    sets.insert("LAZY_EVALUATION".to_string(), serde_json::Value::Bool(true));

    let contract_source_bytes =
      include_bytes!("../../../testdata/contracts/deterministic-fetch.js");
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
            input: serde_json::json!({"id":"8f39fb4940c084460da00a876a521ef2ba84ad6ea8d2f5628c9f1f8aeb395342"}).to_string(),
          }],
          contract_init_state: Some(String::from(include_str!("../../../testdata/contracts/deterministic-fetch.json"))),
          maybe_config: None,
          maybe_cache: Some(false),
          maybe_bundled_contract: None,
          maybe_settings: Some(sets),
          maybe_exm_context: Some((r#"{"requests":{"d1b57dac5f734a8831b808bdd5dfdb9cd0d12a56014190982ce744d99a9a661c":{"type":"basic","url":"https://api.blockcypher.com/v1/eth/main/txs/8f39fb4940c084460da00a876a521ef2ba84ad6ea8d2f5628c9f1f8aeb395342","statusText":"OK","status":127,"redirected":false,"ok":true,"headers":{"x-ratelimit-remaining":"99","access-control-allow-methods":"GET, POST, PUT, DELETE","server":"cloudflare","cf-cache-status":"DYNAMIC","access-control-allow-origin":"*","content-type":"application/json","date":"Tue, 21 Mar 2023 18:14:43 GMT","access-control-allow-headers":"Origin, X-Requested-With, Content-Type, Accept","cf-ray":"7ab82d1aed372937-ORD-X"},"vector":[123,10,32,32,34,98,108,111,99,107,95,104,97,115,104,34,58,32,34,98,51,53,50,54,53,57,54,53,102,54,102,49,52,99,53,100,54,57,50,98,50,48,51,48,57,50,50,102,99,101,53,51,48,54,55,102,99,48,101,57,100,56,99,56,52,57,48,99,51,49,100,52,101,99,100,51,101,52,54,53,48,48,99,34,44,10,32,32,34,98,108,111,99,107,95,104,101,105,103,104,116,34,58,32,49,53,54,52,48,52,48,44,10,32,32,34,98,108,111,99,107,95,105,110,100,101,120,34,58,32,48,44,10,32,32,34,104,97,115,104,34,58,32,34,56,102,51,57,102,98,52,57,52,48,99,48,56,52,52,54,48,100,97,48,48,97,56,55,54,97,53,50,49,101,102,50,98,97,56,52,97,100,54,101,97,56,100,50,102,53,54,50,56,99,57,102,49,102,56,97,101,98,51,57,53,51,52,50,34,44,10,32,32,34,97,100,100,114,101,115,115,101,115,34,58,32,91,10,32,32,32,32,34,52,101,57,100,56,98,52,102,49,56,100,57,56,52,102,54,102,48,99,56,56,100,48,55,101,52,98,51,57,50,48,49,101,56,50,53,99,100,49,55,34,44,10,32,32,32,32,34,55,51,56,100,49,52,53,102,97,97,98,98,49,101,48,48,99,102,53,97,48,49,55,53,56,56,97,57,99,48,102,57,57,56,51,49,56,48,49,50,34,10,32,32,93,44,10,32,32,34,116,111,116,97,108,34,58,32,49,48,49,53,51,49,53,51,51,53,57,52,51,55,51,50,54,44,10,32,32,34,102,101,101,115,34,58,32,49,53,57,53,53,56,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,115,105,122,101,34,58,32,49,49,54,44,10,32,32,34,103,97,115,95,108,105,109,105,116,34,58,32,53,48,48,48,48,48,44,10,32,32,34,103,97,115,95,117,115,101,100,34,58,32,55,57,55,55,57,44,10,32,32,34,103,97,115,95,112,114,105,99,101,34,58,32,50,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,103,97,115,95,116,105,112,95,99,97,112,34,58,32,50,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,103,97,115,95,102,101,101,95,99,97,112,34,58,32,50,48,48,48,48,48,48,48,48,48,48,44,10,32,32,34,99,111,110,102,105,114,109,101,100,34,58,32,34,50,48,49,54,45,48,53,45,50,50,84,49,50,58,52,51,58,48,48,90,34,44,10,32,32,34,114,101,99,101,105,118,101,100,34,58,32,34,50,48,49,54,45,48,53,45,50,50,84,49,50,58,52,51,58,48,48,90,34,44,10,32,32,34,118,101,114,34,58,32,48,44,10,32,32,34,100,111,117,98,108,101,95,115,112,101,110,100,34,58,32,102,97,108,115,101,44,10,32,32,34,118,105,110,95,115,122,34,58,32,49,44,10,32,32,34,118,111,117,116,95,115,122,34,58,32,49,44,10,32,32,34,105,110,116,101,114,110,97,108,95,116,120,105,100,115,34,58,32,91,10,32,32,32,32,34,100,100,49,48,55,99,56,52,56,56,56,54,55,102,100,53,51,99,48,97,97,51,98,102,49,100,56,97,52,55,56,97,48,55,55,101,99,54,55,97,102,55,53,56,52,50,100,50,52,102,49,97,54,52,101,98,52,52,101,52,100,57,48,50,34,44,10,32,32,32,32,34,57,97,57,56,54,55,56,100,50,48,57,57,49,48,102,55,48,98,100,101,100,54,50,52,102,50,99,53,99,49,56,101,100,49,55,100,52,49,55,55,53,50,48,55,50,100,52,49,99,53,100,54,49,98,52,50,99,50,55,55,51,52,56,99,34,44,10,32,32,32,32,34,97,100,57,53,57,57,54,49,102,52,54,54,54,51,102,55,56,53,101,49,101,48,97,48,98,50,54,53,54,57,54,48,98,53,100,50,55,48,54,49,48,57,55,99,53,48,57,50,99,52,101,54,56,57,54,52,98,102,97,102,48,48,52,49,34,44,10,32,32,32,32,34,53,97,50,51,100,55,52,97,52,56,52,101,99,53,56,98,50,49,49,50,54,52,56,56,56,56,52,54,98,101,54,100,55,52,50,56,99,51,51,98,101,57,51,50,50,51,51,57,54,101,102,102,51,54,50,54,101,48,51,54,97,55,102,52,34,44,10,32,32,32,32,34,55,51,57,48,54,98,50,102,49,97,49,100,55,102,100,49,55,99,55,55,54,56,52,101,53,101,56,49,49,97,101,56,55,49,48,101,52,97,48,51,50,53,48,49,48,57,48,100,97,50,54,52,55,49,50,100,98,55,97,52,56,101,55,97,34,44,10,32,32,32,32,34,53,51,99,101,56,56,101,49,99,102,57,56,98,51,55,102,98,54,52,99,49,54,49,50,49,98,52,54,52,49,100,100,101,54,53,50,57,48,56,51,56,99,101,48,98,100,101,54,99,98,49,99,51,49,57,100,102,51,101,102,56,102,102,57,34,44,10,32,32,32,32,34,50,57,98,99,52,101,55,102,97,50,100,98,50,56,98,48,97,50,101,102,57,101,55,97,53,101,48,55,51,99,99,55,53,51,50,101,54,48,57,102,55,53,50,98,50,101,98,48,98,53,55,51,49,98,48,99,54,98,57,50,54,97,50,99,34,44,10,32,32,32,32,34,52,97,57,97,102,99,53,97,54,56,48,57,49,55,57,53,53,101,55,56,49,98,56,57,48,54,56,56,49,52,51,102,53,54,98,97,100,99,55,54,55,56,53,97,102,57,56,55,53,51,100,53,50,54,55,50,55,48,100,55,100,56,48,57,34,10,32,32,93,44,10,32,32,34,99,111,110,102,105,114,109,97,116,105,111,110,115,34,58,32,49,53,51,49,51,54,56,48,44,10,32,32,34,99,111,110,102,105,100,101,110,99,101,34,58,32,49,44,10,32,32,34,105,110,112,117,116,115,34,58,32,91,10,32,32,32,32,123,10,32,32,32,32,32,32,34,115,101,113,117,101,110,99,101,34,58,32,50,55,51,44,10,32,32,32,32,32,32,34,97,100,100,114,101,115,115,101,115,34,58,32,91,10,32,32,32,32,32,32,32,32,34,55,51,56,100,49,52,53,102,97,97,98,98,49,101,48,48,99,102,53,97,48,49,55,53,56,56,97,57,99,48,102,57,57,56,51,49,56,48,49,50,34,10,32,32,32,32,32,32,93,10,32,32,32,32,125,10,32,32,93,44,10,32,32,34,111,117,116,112,117,116,115,34,58,32,91,10,32,32,32,32,123,10,32,32,32,32,32,32,34,118,97,108,117,101,34,58,32,49,48,49,53,51,49,53,51,51,53,57,52,51,55,51,50,54,44,10,32,32,32,32,32,32,34,115,99,114,105,112,116,34,58,32,34,52,101,55,49,100,57,50,100,34,44,10,32,32,32,32,32,32,34,97,100,100,114,101,115,115,101,115,34,58,32,91,10,32,32,32,32,32,32,32,32,34,52,101,57,100,56,98,52,102,49,56,100,57,56,52,102,54,102,48,99,56,56,100,48,55,101,52,98,51,57,50,48,49,101,56,50,53,99,100,49,55,34,10,32,32,32,32,32,32,93,10,32,32,32,32,125,10,32,32,93,10,125]}}}"#).to_string()),
          maybe_contract_source: Some(ContractSource {
            contract_src: contract_source_vec.into(),
            contract_type: SimulateContractType::JAVASCRIPT,
          }),
        };

    let contract = simulate_contract(execution_context).await.unwrap();
    println!("{}", contract.state);
  }
}
