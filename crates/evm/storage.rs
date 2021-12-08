// Contract storage
use primitive_types::U256;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Storage {
  pub inner: HashMap<U256, HashMap<U256, U256>>,
}

impl Storage {
  pub fn new(owner: U256) -> Self {
    let mut inner = HashMap::new();
    inner.insert(owner, HashMap::new());

    Storage { inner }
  }

  pub fn insert(&mut self, account: &U256, key: U256, value: U256) {
    let account = self.inner.get_mut(account).unwrap();
    account.insert(key, value);
  }

  pub fn get(&self, account: &U256, key: &U256) -> U256 {
    let account = self.inner.get(account).unwrap();
    *account.get(key).unwrap_or(&U256::zero())
  }
}
