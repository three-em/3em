use crate::arweave::LoadedContract;
use crate::cache::StateResult;
use crate::cache::CacheExt;
use crate::gql_result::GQLEdgeInterface;
use lru::LruCache;

#[derive(Debug)]
pub struct ArweaveLruCache {
  contracts: LruCache<String, LoadedContract>,
  interactions: LruCache<String, Vec<GQLEdgeInterface>>,
  states: LruCache<String, StateResult>,
}

impl CacheExt for ArweaveLruCache {
  fn new() -> ArweaveLruCache {
    ArweaveLruCache {
      contracts: LruCache::unbounded(),
      interactions: LruCache::unbounded(),
      states: LruCache::unbounded(),
    }
  }

  fn find_contract(
    &mut self,
    contract_id: String,
  ) -> Option<LoadedContract> {
    self.contracts.get_mut(&contract_id).cloned()
  }

  fn find_interactions(
    &mut self,
    contract_id: String,
  ) -> Option<Vec<GQLEdgeInterface>> {
    self.interactions.get_mut(&contract_id).cloned()
  }

  fn find_state(&mut self, contract_id: String) -> Option<StateResult> {
    self.states.get_mut(&contract_id).cloned()
  }

  fn cache_contract(&mut self, loaded_contract: &LoadedContract) {
    self
      .contracts
      .put(loaded_contract.id.to_owned(), loaded_contract.clone());
  }

  fn cache_interactions(
    &mut self,
    contract_id: String,
    interactions: &[GQLEdgeInterface],
  ) {
    self.interactions.put(contract_id, interactions.to_vec());
  }

  fn cache_states(
    &mut self,
    contract_id: String,
    state: StateResult,
  ) {
    self.states.put(contract_id, state);
  }
}
