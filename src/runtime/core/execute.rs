use crate::runtime::core::arweave::Arweave;
use crate::runtime::core::gql_result::{
  GQLEdgeInterface, GQLNodeInterface, GQLTagInterface,
};
use crate::runtime::core::miscellaneous::{get_sort_key, ContractType};
use crate::runtime::smartweave::{ContractBlock, ContractInfo};
use crate::runtime::wasm::WasmRuntime;
use crate::runtime::Runtime;
use deno_core::error::AnyError;
use deno_core::serde_json::Value;
use serde_json::value::Value::Null;
use std::collections::HashMap;
use std::time::Instant;

struct ContractHandlerResult {
  result: Option<Value>,
  state: Option<Value>,
}

pub async fn execute_contract(
  arweave: Arweave,
  contract_id: String,
  contract_src_tx: Option<String>,
  contract_content_type: Option<String>,
  height: Option<usize>,
) {
  let shared_id = contract_id.clone();
  let shared_client = arweave.clone();

  let (loaded_contract, interactions) = tokio::join!(
    tokio::spawn(async move {
      shared_client
        .load_contract(shared_id, contract_src_tx, contract_content_type)
        .await
    }),
    tokio::spawn(
      async move { arweave.get_interactions(contract_id, height).await }
    )
  );

  let loaded_contract = loaded_contract.unwrap();
  let mut interactions = interactions.unwrap();

  interactions.sort_by(|a, b| {
    let a_sort_key =
      get_sort_key(&a.node.block.height, &a.node.block.id, &a.node.id);
    let b_sort_key =
      get_sort_key(&b.node.block.height, &b.node.block.id, &b.node.id);
    a_sort_key.cmp(&b_sort_key)
  });

  let mut validity: HashMap<String, bool> = HashMap::new();

  // Todo: handle wasm, evm, etc.
  match loaded_contract.contract_type {
    ContractType::JAVASCRIPT => {
      let mut state: Value =
        deno_core::serde_json::from_str(&loaded_contract.init_state).unwrap();

      let mut rt = Runtime::new(
        &(String::from_utf8(loaded_contract.contract_src).unwrap()),
        state,
      )
      .await
      .unwrap();

      for interaction in interactions {
        let tx = interaction.node;
        let input = get_input_from_interaction(&tx);

        // TODO: has_multiple_interactions https://github.com/ArweaveTeam/SmartWeave/blob/4d09c66d832091805f583ba73e8da96cde2c0190/src/contract-read.ts#L68
        let js_input: Value = deno_core::serde_json::from_str(input).unwrap();

        let call_input = serde_json::json!({
          "input": js_input,
          "caller": tx.owner.address
        });

        let valid = rt.call(call_input).await.is_ok();
        validity.insert(tx.id, valid);
      }
    }
    ContractType::WASM => {
      let wasm = loaded_contract.contract_src.as_slice();
      let transaction = loaded_contract.contract_transaction;
      let contract_info = ContractInfo {
        transaction,
        block: ContractBlock {
          height: 0,
          indep_hash: String::from(""),
          timestamp: String::from(""),
        },
      };
      let mut state: Value =
        deno_core::serde_json::from_str(&loaded_contract.init_state).unwrap();
      let mut rt = WasmRuntime::new(wasm, contract_info).await.unwrap();

      for interaction in interactions {
        let tx = interaction.node;
        let input = get_input_from_interaction(&tx);
        let wasm_input: Value = deno_core::serde_json::from_str(input).unwrap();

        let mut prev_state = deno_core::serde_json::to_vec(&state).unwrap();
        let call_input = serde_json::json!({
          "input": wasm_input,
          "caller": tx.owner.address,
        });

        let mut input = deno_core::serde_json::to_vec(&call_input).unwrap();
        let exec = rt.call(&mut prev_state, &mut input).await;
        let valid = exec.is_ok();
        if valid {
          state = deno_core::serde_json::from_slice(&(exec.unwrap())).unwrap();
        }
        validity.insert(tx.id, valid);
      }
    }
    ContractType::EVM => {}
  }
}

pub fn get_input_from_interaction(interaction_tx: &GQLNodeInterface) -> &str {
  let tag = (&(&interaction_tx)
    .tags
    .iter()
    .find(|data| &data.name == "Input"));

  match tag {
    Some(data) => &data.value,
    None => "",
  }
}

pub fn has_multiple_interactions(interaction_tx: &GQLNodeInterface) -> bool {
  let tags = (&interaction_tx.tags).to_owned();
  let filtered_tags = tags
    .iter()
    .filter(|data| data.name == String::from("Contract"))
    .cloned()
    .collect::<Vec<GQLTagInterface>>();
  filtered_tags.len() > 1
}
