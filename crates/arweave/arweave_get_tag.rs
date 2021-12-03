use crate::arweave::TransactionData;
use crate::utils::decode_base_64;

pub fn get_tag(transaction: &TransactionData, tag: &str) -> String {
  let tags = &transaction.tags;

  let maybe_tag = tags.iter().find(|data| {
    let name = &data.name;
    let decoded_name = decode_base_64(name.to_owned());
    decoded_name == String::from(tag)
  });

  match maybe_tag {
    Some(data) => decode_base_64(String::from(&data.value)),
    None => String::from(""),
  }
}
