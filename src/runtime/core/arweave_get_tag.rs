use crate::runtime::core::arweave::TransactionData;

pub fn get_tag(transaction: &TransactionData, tag: &str) -> String {
  // TODO: DECODE & STRING. ARE THEY NEEDED ?

  let tags = &transaction.tags;

  let maybe_tag = tags.iter().find(|data| data.name == String::from(tag));

  match maybe_tag {
    Some(data) => String::from(&data.value),
    None => String::from(""),
  }
}
