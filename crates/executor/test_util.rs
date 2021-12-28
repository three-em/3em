#![cfg(test)]

use deno_core::serde_json::Value;
use three_em_arweave::gql_result::{
  GQLBlockInterface, GQLEdgeInterface, GQLNodeInterface, GQLOwnerInterface,
  GQLTagInterface,
};

pub fn generate_fake_interaction(
  input: Value,
  id: &str,
  block_id: Option<String>,
  block_height: Option<usize>,
) -> GQLEdgeInterface {
  GQLEdgeInterface {
    cursor: String::new(),
    node: GQLNodeInterface {
      id: String::from(id),
      anchor: None,
      signature: None,
      recipient: None,
      owner: GQLOwnerInterface {
        address: String::new(),
        key: None,
      },
      fee: None,
      quantity: None,
      data: None,
      tags: vec![GQLTagInterface {
        name: String::from("Input"),
        value: input.to_string(),
      }],
      block: GQLBlockInterface {
        id: block_id.unwrap_or(String::new()),
        timestamp: 0,
        height: block_height.unwrap_or(0),
        previous: None,
      },
      parent: None,
      bundledIn: None,
    },
  }
}
