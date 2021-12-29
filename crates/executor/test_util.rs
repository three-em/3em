use deno_core::serde_json::Value;
use three_em_arweave::gql_result::{
  GQLAmountInterface, GQLBlockInterface, GQLEdgeInterface, GQLNodeInterface,
  GQLOwnerInterface, GQLTagInterface,
};

pub fn generate_fake_interaction(
  input: Value,
  id: &str,
  block_id: Option<String>,
  block_height: Option<usize>,
  owner_address: Option<String>,
  recipient: Option<String>,
  extra_tag: Option<GQLTagInterface>,
  quantity: Option<GQLAmountInterface>,
  fee: Option<GQLAmountInterface>,
  block_timestamp: Option<usize>,
) -> GQLEdgeInterface {
  let mut tags = vec![GQLTagInterface {
    name: String::from("Input"),
    value: input.to_string(),
  }];

  if extra_tag.is_some() {
    tags.push(extra_tag.unwrap());
  }

  GQLEdgeInterface {
    cursor: String::new(),
    node: GQLNodeInterface {
      id: String::from(id),
      anchor: None,
      signature: None,
      recipient,
      owner: GQLOwnerInterface {
        address: owner_address.unwrap_or(String::new()),
        key: None,
      },
      fee,
      quantity,
      data: None,
      tags,
      block: GQLBlockInterface {
        id: block_id.unwrap_or(String::new()),
        timestamp: block_timestamp.unwrap_or(0),
        height: block_height.unwrap_or(0),
        previous: None,
      },
      parent: None,
      bundledIn: None,
    },
  }
}
