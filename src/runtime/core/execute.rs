use crate::runtime::core::arweave::Arweave;
use crate::runtime::core::gql_result::{
  GQLEdgeInterface, GQLNodeInterface, GQLTagInterface,
};
use crate::runtime::core::miscellaneous::ContractType;
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
  arweave: &Arweave,
  contract_id: String,
  contract_src_tx: Option<String>,
  contract_content_type: Option<String>,
  height: Option<usize>,
) {
  let loaded_contract = arweave
    .load_contract(
      contract_id.to_owned(),
      contract_src_tx,
      contract_content_type,
    )
    .await;
  let interactions = arweave
    .get_interactions(contract_id.to_owned(), height)
    .await;

  // TODO: Sort interactions

  // Todo: handle wasm, evm, etc.
  match loaded_contract.contract_type {
    ContractType::JAVASCRIPT => {
      let source = &loaded_contract.contract_src[..];
      let mut rt = Runtime::new(source).await.unwrap();
      let mut state: Value =
        deno_core::serde_json::from_str(&loaded_contract.init_state[..])
          .unwrap();
      let mut validity: HashMap<String, bool> = HashMap::new();

      for interaction in interactions {
        let tx = interaction.node;
        let input = get_input_from_interaction(&tx);

        // TODO: has_multiple_interactions  https://github.com/ArweaveTeam/SmartWeave/blob/4d09c66d832091805f583ba73e8da96cde2c0190/src/contract-read.ts#L68
        let js_input: Value = deno_core::serde_json::from_str(&input).unwrap();

        let call_input = serde_json::json!({
          "input": js_input,
          "caller": tx.owner.address
        });

        let call: Result<Value, AnyError> =
          rt.call(&[state.to_owned(), call_input]).await;

        let mut is_valid = false;

        match call {
          Ok(data) => {
            match data.get("state") {
              Some(data) => {
                state = data.to_owned();
                is_valid = true;
              }
              None => {}
            };
          }
          Err(_) => {}
        }

        validity.insert(tx.id, is_valid);
      }

      // println!("{}", state);
      // println!("Interactions: {}", validity.len());
    }
    _ => {}
  }
}

pub fn get_input_from_interaction(interaction_tx: &GQLNodeInterface) -> String {
  let tag = (&(&interaction_tx)
    .tags
    .iter()
    .find(|data| data.name == String::from("Input")));

  match tag {
    Some(data) => String::from((&data.value)),
    None => String::from(""),
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
