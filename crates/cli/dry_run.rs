use deno_core::error::AnyError;
use indexmap::map::IndexMap;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_arweave::gql_result::{
  GQLAmountInterface, GQLEdgeInterface, GQLTagInterface,
};
use three_em_arweave::miscellaneous::ContractType;
use three_em_executor::executor::{raw_execute_contract, ExecuteResult};
use three_em_executor::test_util::{
  generate_fake_interaction, generate_fake_loaded_contract_data,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RawInteractions {
  id: String,
  caller: String,
  input: Value,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  block_id: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  block_height: Option<usize>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  block_timestamp: Option<usize>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  quantity: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  reward: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  tags: Option<GQLTagInterface>,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  recipient: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DryRunFile {
  contract_type: ContractType,
  contract_source: String,
  initial_state: Value,
  interactions: Vec<RawInteractions>,
}

#[allow(clippy::too_many_arguments)]
pub async fn dry_run_result(
  port: i32,
  host: String,
  protocol: String,
  file: String,
) -> ExecuteResult {
  let dry = read_dry_run_file(file);

  let file =
    std::fs::read(dry.contract_source).expect("File source does not exist");
  let dry_contract = generate_fake_loaded_contract_data(
    file.as_slice(),
    dry.contract_type,
    dry.initial_state.to_string(),
  );

  let interactions = dry
    .interactions
    .iter()
    .map(|data| {
      generate_fake_interaction(
        data.input.to_owned(),
        &(data.id.to_owned())[..],
        data.block_id.to_owned(),
        data.block_height.to_owned(),
        Some(data.caller.to_owned()),
        data.recipient.to_owned(),
        None,
        Some(GQLAmountInterface {
          winston: {
            let quantity = data.quantity.to_owned();
            if quantity.is_some() {
              Some(quantity.unwrap())
            } else {
              Some(String::from("0"))
            }
          },
          ar: None,
        }),
        Some(GQLAmountInterface {
          winston: {
            let reward = data.reward.to_owned();
            if reward.is_some() {
              Some(reward.unwrap())
            } else {
              Some(String::from("0"))
            }
          },
          ar: None,
        }),
        data.block_timestamp,
      )
    })
    .collect::<Vec<GQLEdgeInterface>>();

  let execution = raw_execute_contract(
    String::from(""),
    dry_contract,
    interactions,
    IndexMap::new(),
    None,
    true,
    true,
    |_, _, _| panic!("Unimplemented"),
    &Arweave::new(port, host, protocol, ArweaveCache::new()),
    HashMap::new(),
    None,
  )
  .await;
  execution
}

#[allow(clippy::too_many_arguments)]
pub async fn dry_run(
  port: i32,
  host: String,
  protocol: String,
  pretty_print: bool,
  show_validity: bool,
  file: String,
) -> Result<(), AnyError> {
  let execution = dry_run_result(port, host, protocol, file).await;

  if let ExecuteResult::V8(data) = execution {
    let (state, validity, result) = (
      data.state,
      data.validity,
      data.result.unwrap_or(Value::default()),
    );
    let value = if show_validity {
      serde_json::json!({
          "state": state,
          "validity": validity,
          "result": result
      })
    } else {
      state
    };

    if pretty_print {
      println!("{}", serde_json::to_string_pretty(&value).unwrap());
    } else {
      println!("{}", value);
    }
  } else {
    panic!("Dry run is only implemented for WASM and JS contracts");
  }

  Ok(())
}

fn read_dry_run_file<P: AsRef<Path>>(path: P) -> DryRunFile {
  let data = std::fs::read_to_string(path).expect("Unable to read input file");
  let res: DryRunFile =
    serde_json::from_str(&data).expect("Unable to parse input file");
  res
}

#[cfg(test)]
mod tests {
  use crate::dry_run::{dry_run, dry_run_result};
  use three_em_executor::executor::ExecuteResult;

  #[tokio::test]
  async fn test_dry_run() {
    let execution = dry_run_result(
      443,
      String::from("arweave.net"),
      String::from("https"),
      // Exit cargo directory
      String::from("../../testdata/contracts/dry_run_users_contract.json"),
    )
    .await;

    if let ExecuteResult::V8(data) = execution {
      let value = data.state;
      let validity_table = data.validity;
      assert_eq!(
        value,
        serde_json::json!({
          "users": ["Andres Pirela", "Divy", "Some Other"]
        })
      );
      assert_eq!(
        validity_table.get_index(2).unwrap().1.to_owned(),
        "Error: Invalid operation\n    at handle (file:///main.js:5:12)"
      );
    } else {
      panic!("Unexpected result");
    }
  }
}
