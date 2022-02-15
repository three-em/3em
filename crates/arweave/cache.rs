use crate::arweave::LoadedContract;
use crate::gql_result::GQLEdgeInterface;
use deno_core::serde_json::Value;
use indexmap::map::IndexMap;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs::{create_dir_all, remove_file, File};
use std::io::BufReader;
use std::path::PathBuf;

pub trait CacheExt: Debug {
  fn new() -> Self
  where
    Self: Sized;
  fn find_contract(&mut self, contract_id: String) -> Option<LoadedContract>;
  fn find_interactions(
    &mut self,
    contract_id: String,
  ) -> Option<Vec<GQLEdgeInterface>>;
  fn find_state(&mut self, contract_id: String) -> Option<StateResult>;
  fn cache_contract(&mut self, loaded_contract: &LoadedContract);
  fn cache_interactions(
    &mut self,
    contract_id: String,
    interactions: &[GQLEdgeInterface],
  );
  fn cache_states(&mut self, contract_id: String, state: StateResult);
}

#[derive(Debug)]
pub struct ArweaveCache {
  pub contracts_cache_folder: PathBuf,
  pub interactions_cache_folder: PathBuf,
  pub states_cache_folder: PathBuf,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StateResult {
  pub state: Value,
  pub validity: IndexMap<String, Value>,
}

impl Default for ArweaveCache {
  fn default() -> Self {
    Self::new()
  }
}

impl CacheExt for ArweaveCache {
  fn new() -> ArweaveCache {
    if let Some(cache_dir) = dirs::cache_dir() {
      let root_cache_dir = cache_dir.join("3em").join("contracts");
      let interactions_cache_dir = cache_dir.join("3em").join("interactions");
      let states_cache_dir = cache_dir.join("3em").join("states");

      create_dir_all(root_cache_dir.to_owned()).unwrap();
      create_dir_all(interactions_cache_dir.to_owned()).unwrap();
      create_dir_all(states_cache_dir.to_owned()).unwrap();

      ArweaveCache {
        contracts_cache_folder: root_cache_dir,
        interactions_cache_folder: interactions_cache_dir,
        states_cache_folder: states_cache_dir,
      }
    } else {
      panic!("Cache folder could not be set");
    }
  }

  fn find_contract(&mut self, contract_id: String) -> Option<LoadedContract> {
    let cache_file = self.get_cache_file(contract_id);

    let file = File::open(cache_file);

    match file {
      Ok(data) => {
        let reader = BufReader::new(data);
        let loaded_contract: LoadedContract =
          deno_core::serde_json::from_reader(reader).unwrap();
        Some(loaded_contract)
      }
      Err(_) => None,
    }
  }

  fn find_interactions(
    &mut self,
    contract_id: String,
  ) -> Option<Vec<GQLEdgeInterface>> {
    let cache_file = self.get_cache_interaction_file(contract_id);

    let file = File::open(cache_file);

    match file {
      Ok(data) => {
        let reader = BufReader::new(data);
        let interactions: Vec<GQLEdgeInterface> =
          deno_core::serde_json::from_reader(reader).unwrap();
        Some(interactions)
      }
      Err(_) => None,
    }
  }

  fn find_state(&mut self, contract_id: String) -> Option<StateResult> {
    let cache_file = self.get_cache_state_file(contract_id);

    let file = File::open(cache_file);

    match file {
      Ok(data) => {
        let reader = BufReader::new(data);
        let state: StateResult =
          deno_core::serde_json::from_reader(reader).unwrap();
        Some(state)
      }
      Err(_) => None,
    }
  }

  fn cache_contract(&mut self, loaded_contract: &LoadedContract) {
    let cache_file = self.get_cache_file(loaded_contract.id.to_owned());
    deno_core::serde_json::to_writer(
      &File::create(cache_file).unwrap(),
      loaded_contract,
    )
    .unwrap();
  }

  fn cache_interactions(
    &mut self,
    contract_id: String,
    interactions: &[GQLEdgeInterface],
  ) {
    let cache_file = self.get_cache_interaction_file(contract_id);
    deno_core::serde_json::to_writer(
      &File::create(cache_file).unwrap(),
      interactions,
    )
    .unwrap();
  }

  fn cache_states(&mut self, contract_id: String, state: StateResult) {
    let cache_file = self.get_cache_state_file(contract_id);
    deno_core::serde_json::to_writer(
      &File::create(cache_file).unwrap(),
      &state,
    )
    .unwrap();
  }
}

impl ArweaveCache {
  pub async fn delete_cache_interactions(&self, contract_id: String) {
    let cache_file = self.get_cache_interaction_file(contract_id);
    remove_file(cache_file).unwrap();
  }

  fn get_cache_file(&self, contract_id: String) -> PathBuf {
    let mut cache_file = self.contracts_cache_folder.to_owned();

    cache_file.push(format!("{}.json", contract_id));

    cache_file
  }

  fn get_cache_interaction_file(&self, contract_id: String) -> PathBuf {
    let mut cache_file = self.interactions_cache_folder.to_owned();

    cache_file.push(format!("{}.json", contract_id));

    cache_file
  }

  fn get_cache_state_file(&self, contract_id: String) -> PathBuf {
    let mut cache_file = self.states_cache_folder.to_owned();

    cache_file.push(format!("{}_result.json", contract_id));

    cache_file
  }
}
