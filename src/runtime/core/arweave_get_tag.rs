use crate::runtime::core::arweave::TransactionData;
use crate::utils::decode_base_64;

pub fn get_tag(transaction: &TransactionData, tag: &str) -> String {
  // TODO: DECODE & STRING. ARE THEY NEEDED ?

  let tags = &transaction.tags;

  let maybe_tag = tags.iter().find(|data| {
    let name = &data.name;
    let decoded_name =
      String::from_utf8(base64::decode(name).unwrap_or_else(|_| vec![]))
        .unwrap();
    decoded_name == String::from(tag)
  });

  match maybe_tag {
    Some(data) => decode_base_64(String::from(&data.value)),
    None => String::from(""),
  }
}
