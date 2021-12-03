use crate::runtime::core::arweave::TransactionData;
use crate::runtime::core::arweave_get_tag::get_tag;
use crate::utils::hasher;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub enum ContractType {
  JAVASCRIPT,
  WASM,
  EVM,
}

pub fn get_contract_type(
  maybe_content_type: Option<String>,
  contract_transaction: &TransactionData,
  source_transaction: &TransactionData,
) -> ContractType {
  let mut contract_type = maybe_content_type
    .unwrap_or_else(|| get_tag(&source_transaction, "Content-Type"));
  if contract_type.len() <= 0 {
    contract_type = get_tag(&contract_transaction, "Content-Type");
  }

  match &(contract_type.to_lowercase())[..] {
    "application/javascript" => ContractType::JAVASCRIPT,
    "application/wasm" => ContractType::WASM,
    "application/evm" => ContractType::EVM,
    _ => ContractType::JAVASCRIPT,
  }
}

pub fn get_sort_key(
  block_height: &usize,
  block_id: &String,
  transaction_id: &String,
) -> String {
  let mut hasher_bytes = block_id.to_owned().into_bytes();
  hasher_bytes.append(&mut transaction_id.to_owned().into_bytes());
  let hashed = hex::encode(hasher(&hasher_bytes[..]));
  let height = format!("000000{}", block_height);

  format!("{},{}", height, hashed)
}

#[cfg(test)]
mod tests {
  use crate::runtime::core::arweave::{Tag, TransactionData};
  use crate::runtime::core::miscellaneous::{get_contract_type, ContractType};

  #[tokio::test]
  async fn get_contract_type_test() {
    let contract_type = get_contract_type(
      Some(String::from("invalid")),
      &get_fake_transaction("whatever"),
      &get_fake_transaction("whatever"),
    );
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction("whatever"),
      &get_fake_transaction("whatever"),
    );
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction(""),
      &get_fake_transaction("whatever"),
    );
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction("whatever"),
      &get_fake_transaction("application/wasm"),
    );
    assert!(matches!(contract_type, ContractType::WASM));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction(""),
      &get_fake_transaction("application/evm"),
    );
    assert!(matches!(contract_type, ContractType::EVM));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction(""),
      &get_fake_transaction(""),
    );
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
  }

  fn get_fake_transaction(content_type: &str) -> TransactionData {
    TransactionData {
      format: 1 as usize,
      id: String::from(""),
      last_tx: String::from(""),
      owner: String::from(""),
      tags: vec![Tag {
        name: String::from("Q29udGVudC1UeXBl"),
        value: base64::encode(String::from(content_type)),
      }],
      target: String::from(""),
      quantity: String::from(""),
      data: String::from(""),
      reward: String::from(""),
      signature: String::from(""),
      data_size: String::from(""),
      data_root: String::from(""),
    }
  }
}
