use deno_core::serde_json::Value;
use three_em_arweave::gql_result::{
  GQLAmountInterface, GQLBlockInterface, GQLEdgeInterface, GQLNodeInterface,
  GQLOwnerInterface, GQLTagInterface,
};

const ONE_AR: f64 = (1000000000000 as i64) as f64;
pub fn ar_to_winston(ar: f64) -> f64 {
  ar * ONE_AR
}

pub fn winston_to_ar(winston: f64) -> f64 {
  winston / ONE_AR
}

pub fn create_simulated_transaction(
  id: String,
  owner: String,
  quantity: String,
  reward: String,
  target: Option<String>,
  tags: Vec<GQLTagInterface>,
  block_height: Option<String>,
  block_hash: Option<String>,
  block_timestamp: Option<String>,
  input: String,
) -> GQLEdgeInterface {
  let mut current_tags = tags;
  current_tags.push(GQLTagInterface {
    name: String::from("Input"),
    value: input,
  });

  let height = block_height
    .unwrap_or_else(|| String::from("0"))
    .parse::<usize>()
    .unwrap_or(0 as usize);
  let timestamp = block_timestamp
    .unwrap_or_else(|| String::from("0"))
    .parse::<usize>()
    .unwrap_or(0 as usize);
  GQLEdgeInterface {
    cursor: String::new(),
    node: GQLNodeInterface {
      id,
      anchor: None,
      signature: None,
      recipient: target,
      owner: GQLOwnerInterface {
        address: owner,
        key: None,
      },
      quantity: Some(GQLAmountInterface {
        winston: Some(quantity),
        ar: None,
      }),
      fee: Some(GQLAmountInterface {
        winston: Some(reward),
        ar: None,
      }),
      data: None,
      tags: current_tags,
      block: GQLBlockInterface {
        height,
        timestamp,
        id: block_hash.unwrap_or_else(|| String::new()),
        previous: None,
      },
      parent: None,
      bundledIn: None,
    },
  }
}
