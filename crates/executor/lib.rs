pub mod executor;

use crate::executor::raw_execute_contract;
use crate::executor::ExecuteResult;
use deno_core::error::AnyError;
use deno_core::serde_json::Value;
use std::collections::HashMap;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::arweave::LoadedContract;
use three_em_arweave::arweave::ARWEAVE_CACHE;
use three_em_arweave::gql_result::GQLEdgeInterface;
use three_em_arweave::gql_result::GQLNodeInterface;
use three_em_arweave::gql_result::GQLTagInterface;
use three_em_arweave::miscellaneous::get_sort_key;
use three_em_evm::Instruction;
use three_em_evm::U256;

pub async fn execute_contract(
  arweave: Arweave,
  contract_id: String,
  contract_src_tx: Option<String>,
  contract_content_type: Option<String>,
  height: Option<usize>,
  cache: bool,
) -> Result<ExecuteResult, AnyError> {
  let contract_id_copy = contract_id.to_owned();
  let shared_id = contract_id.clone();
  let shared_client = arweave.clone();
  let shared_client2 = arweave.clone();
  let (loaded_contract, interactions) = tokio::join!(
    tokio::spawn(async move {
      let contract: Result<LoadedContract, AnyError> = shared_client
        .load_contract(shared_id, contract_src_tx, contract_content_type, cache)
        .await;

      contract
    }),
    tokio::spawn(async move {
      let interactions: Result<(Vec<GQLEdgeInterface>, usize, bool), AnyError> =
        shared_client2
          .get_interactions(contract_id, height, cache)
          .await;
      let (
        result_interactions,
        new_interaction_index,
        are_there_new_interactions,
      ) = interactions?;

      let mut interactions = result_interactions;

      interactions.sort_by(|a, b| {
        let a_sort_key =
          get_sort_key(&a.node.block.height, &a.node.block.id, &a.node.id);
        let b_sort_key =
          get_sort_key(&b.node.block.height, &b.node.block.id, &b.node.id);
        a_sort_key.cmp(&b_sort_key)
      });

      Ok((
        interactions,
        new_interaction_index,
        are_there_new_interactions,
      )) as Result<(Vec<GQLEdgeInterface>, usize, bool), AnyError>
    })
  );

  let loaded_contract = loaded_contract?.unwrap();
  let (result_interactions, new_interaction_index, are_there_new_interactions) =
    interactions?.unwrap();

  let mut interactions = result_interactions;

  let mut validity: HashMap<String, bool> = HashMap::new();

  let mut needs_processing = true;
  let mut cache_state: Option<Value> = None;

  if cache {
    let get_cached_state =
      ARWEAVE_CACHE.find_state(contract_id_copy.to_owned()).await;

    if let Some(cached_state) = get_cached_state {
      cache_state = Some(cached_state.state);
      validity = cached_state.validity;
      needs_processing = are_there_new_interactions;
    }
  }

  let is_cache_state_present = cache_state.is_some();

  if cache && is_cache_state_present && are_there_new_interactions {
    interactions = (&interactions[new_interaction_index..]).to_vec();
  }

  Ok(
    raw_execute_contract(
      contract_id_copy.to_owned(),
      loaded_contract,
      interactions,
      validity,
      cache_state,
      needs_processing,
      |validity_table, cache_state| {
        ExecuteResult::V8(cache_state.unwrap(), validity_table)
      },
      arweave,
    )
    .await,
  )
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

fn nop_cost_fn(_: &Instruction) -> U256 {
  U256::zero()
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
      false,
    )
    .await
    .unwrap();
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
      false,
    )
    .await
    .unwrap();
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
