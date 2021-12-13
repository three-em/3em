use crate::arweave::TransactionData;
use crate::utils::hasher;
use deno_core::error::AnyError;
use serde::Deserialize;
use serde::Serialize;

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
) -> Result<ContractType, AnyError> {
  let contract_type = maybe_content_type
    .or_else(|| source_transaction.get_tag("Content-Type").ok())
    .or_else(|| contract_transaction.get_tag("Content-Type").ok())
    .ok_or(AnyError::msg("Contract-Src tag not found in transaction"))?;

  let ty = match &(contract_type.to_lowercase())[..] {
    "application/javascript" => ContractType::JAVASCRIPT,
    "application/wasm" => ContractType::WASM,
    "application/octet-stream" => ContractType::EVM,
    _ => ContractType::JAVASCRIPT,
  };

  Ok(ty)
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
  use crate::arweave::{Tag, TransactionData};
  use crate::miscellaneous::{get_contract_type, ContractType};

  #[tokio::test]
  async fn get_contract_type_test() {
    let contract_type = get_contract_type(
      Some(String::from("invalid")),
      &get_fake_transaction("whatever"),
      &get_fake_transaction("whatever"),
    )
    .unwrap();
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction("whatever"),
      &get_fake_transaction("whatever"),
    )
    .unwrap();
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction(""),
      &get_fake_transaction("whatever"),
    )
    .unwrap();
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction("whatever"),
      &get_fake_transaction("application/wasm"),
    )
    .unwrap();
    assert!(matches!(contract_type, ContractType::WASM));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction(""),
      &get_fake_transaction("application/octet-stream"),
    )
    .unwrap();
    assert!(matches!(contract_type, ContractType::EVM));
    let contract_type = get_contract_type(
      None,
      &get_fake_transaction(""),
      &get_fake_transaction(""),
    )
    .unwrap();
    assert!(matches!(contract_type, ContractType::JAVASCRIPT));
  }

  fn get_fake_transaction(content_type: &str) -> TransactionData {
    TransactionData {
      format: 1_usize,
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
