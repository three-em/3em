use deno_core::error::AnyError;
use deno_core::serde_json;
use deno_core::serde_json::Value;
use serde_json::value::Value::Null;
use std::collections::HashMap;
use std::time::Instant;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::gql_result::{
  GQLEdgeInterface, GQLNodeInterface, GQLTagInterface,
};
use three_em_arweave::miscellaneous::get_sort_key;
use three_em_arweave::miscellaneous::ContractType;
use three_em_js::Runtime;
use three_em_smartweave::ContractBlock;
use three_em_smartweave::ContractInfo;
use three_em_wasm::WasmRuntime;

struct ContractHandlerResult {
  result: Option<Value>,
  state: Option<Value>,
}

pub type ValidityTable = HashMap<String, bool>;

pub enum ExecuteResult {
  V8(Value, ValidityTable),
  Evm(Vec<u8>, ValidityTable),
}

pub async fn execute_contract(
  arweave: Arweave,
  contract_id: String,
  contract_src_tx: Option<String>,
  contract_content_type: Option<String>,
  height: Option<usize>,
) -> ExecuteResult {
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
  let transaction = loaded_contract.contract_transaction;
  let contract_info = ContractInfo {
    transaction,
    block: ContractBlock {
      height: 0,
      indep_hash: String::from(""),
      timestamp: String::from(""),
    },
  };

  // TODO: handle evm.
  match loaded_contract.contract_type {
    ContractType::JAVASCRIPT => {
      let mut state: Value =
        deno_core::serde_json::from_str(&loaded_contract.init_state).unwrap();

      let mut rt = Runtime::new(
        &(String::from_utf8(loaded_contract.contract_src).unwrap()),
        state,
        contract_info,
      )
      .await
      .unwrap();

      for interaction in interactions {
        let tx = interaction.node;
        let input = get_input_from_interaction(&tx);

        // TODO: has_multiple_interactions
        // https://github.com/ArweaveTeam/SmartWeave/blob/4d09c66d832091805f583ba73e8da96cde2c0190/src/contract-read.ts#L68
        let js_input: Value = deno_core::serde_json::from_str(input).unwrap();

        let call_input = serde_json::json!({
          "input": js_input,
          "caller": tx.owner.address
        });

        let valid = rt.call(call_input).await.is_ok();
        validity.insert(tx.id, valid);
      }

      let state_val: Value = rt.get_contract_state().unwrap();
      ExecuteResult::V8(state_val, validity)
    }
    ContractType::WASM => {
      let wasm = loaded_contract.contract_src.as_slice();

      let mut state: Value =
        deno_core::serde_json::from_str(&loaded_contract.init_state).unwrap();
      let mut rt = WasmRuntime::new(wasm, contract_info).unwrap();

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
        let exec = rt.call(&mut prev_state, &mut input);
        let valid = exec.is_ok();
        if valid {
          state = deno_core::serde_json::from_slice(&(exec.unwrap())).unwrap();
        }
        validity.insert(tx.id, valid);
      }

      ExecuteResult::V8(state, validity)
    }
    ContractType::EVM => ExecuteResult::V8(Null, validity),
  }
}

pub fn get_input_from_interaction(interaction_tx: &GQLNodeInterface) -> &str {
  let tag = &(&interaction_tx)
    .tags
    .iter()
    .find(|data| &data.name == "Input");

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

#[cfg(test)]
mod test {
  use crate::execute_contract;
  use crate::ExecuteResult;
  use deno_core::serde_json;
  use serde::Deserialize;
  use serde::Serialize;
  use three_em_arweave::arweave::Arweave;

  #[derive(Deserialize, Serialize)]
  struct People {
    username: String,
  }

  #[tokio::test]
  async fn test_execute_wasm() {
    let arweave = Arweave::new(80, String::from("arweave.net"));
    let result = execute_contract(
      arweave,
      String::from("KfU_1Uxe3-h2r3tP6ZMfMT-HBFlM887tTFtS-p4edYQ"),
      None,
      None,
      Some(822062),
    )
    .await;
    if let ExecuteResult::V8(value, validity) = result {
      assert!(!(value.is_null()));
      assert!(value.get("counter").is_some());
      let counter = value.get("counter").unwrap().as_i64().unwrap();
      assert_eq!(counter, 2);
      assert!(validity
        .get("HBHsDDeWrEmAlkg_mFzYjOsEgG3I6j4id_Aqd1fERgA")
        .is_some());
      assert!(validity
        .get("IlAr0h0rl7oI7FesF1Oy-E_a-K6Al4Avc2pu6CEZkog")
        .is_some());
    } else {
      assert!(false);
    }
  }

  #[tokio::test]
  async fn test_execute_javascript() {
    let arweave = Arweave::new(80, String::from("arweave.net"));
    let result = execute_contract(
      arweave,
      String::from("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE"),
      None,
      None,
      None,
    )
    .await;
    if let ExecuteResult::V8(value, validity) = result {
      assert!(!(value.is_null()));
      assert!(value.get("people").is_some());
      assert!(value.get("people").unwrap().is_array());
      let people = value.get("people").unwrap();
      let people_struct: Vec<People> =
        serde_json::from_value(people.to_owned()).unwrap();
      let is_marton_here = people_struct
        .iter()
        .find(|data| data.username == String::from("martonlederer"));
      assert!(is_marton_here.is_some());
    } else {
      assert!(false);
    }
  }
}
