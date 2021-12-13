// Contract storage
use primitive_types::U256;
use std::collections::HashMap;

macro_rules! extend_u256 {
  ($vec:ident, $val:expr) => {
    let array: [u8; 32] = $val.into();
    $vec.extend_from_slice(&array);
  };
}

#[derive(Debug, Clone)]
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

  /// Decode storage bytes.
  pub fn from_raw(raw: &[u8]) -> Self {
    let mut storage = Storage::new(U256::zero());

    let mut offset = 0;
    while offset < raw.len() {
      let account = U256::from(&raw[offset..offset + 32]);
      offset += 32;

      let mut account_storage = HashMap::new();
      let key_count = U256::from(&raw[offset..offset + 32]);
      offset += 32;

      for _ in 0..key_count.as_usize() {
        let key = U256::from(&raw[offset..offset + 32]);
        offset += 32;

        let value = U256::from(&raw[offset..offset + 32]);
        offset += 32;

        account_storage.insert(key, value);
      }

      storage.inner.insert(account, account_storage);
    }

    storage
  }

  pub fn raw(&self) -> Vec<u8> {
    let mut raw: Vec<u8> = Vec::new();

    for (account, account_storage) in self.inner.iter() {
      extend_u256!(raw, *account);
      extend_u256!(raw, U256::from(account_storage.len()));

      for (key, value) in account_storage.iter() {
        extend_u256!(raw, *key);
        extend_u256!(raw, *value);
      }
    }

    raw
  }
}

#[cfg(test)]
mod tests {
  use crate::storage::Storage;
  use primitive_types::U256;

  #[test]
  fn test_storage_decode() {
    let account = U256::zero();
    let encoded: [[u8; 32]; 6] = [
      // Account
      account.into(),
      // Key count
      U256::from(0x02u8).into(),
      // Key 1
      U256::zero().into(),
      // Value 1
      U256::one().into(),
      // Key 2
      U256::one().into(),
      // Value 2
      U256::from(0x02u8).into(),
    ];

    let store = Storage::from_raw(&encoded.concat());

    assert_eq!(store.get(&account, &U256::zero()), U256::one());
    assert_eq!(store.get(&account, &U256::one()), U256::from(0x02u8));
  }
}
