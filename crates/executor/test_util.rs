use deno_core::serde_json::Value;
use three_em_arweave::arweave::{LoadedContract, TransactionData};
use three_em_arweave::gql_result::{
  GQLAmountInterface, GQLBlockInterface, GQLEdgeInterface, GQLNodeInterface,
  GQLOwnerInterface, GQLTagInterface,
};
use three_em_arweave::miscellaneous::ContractType;

pub fn generate_fake_interaction_input_string(
  input: &str,
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
    value: String::from(input),
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

pub fn generate_fake_loaded_contract_data(
  contract_source: &[u8],
  contract_type: ContractType,
  init_state: String,
) -> LoadedContract {
  let contract_id = String::from("test");
  LoadedContract {
    id: contract_id,
    contract_src_tx_id: String::new(),
    contract_src: contract_source.to_vec(),
    contract_type,
    init_state,
    min_fee: None,
    contract_transaction: TransactionData {
      format: 0,
      id: String::new(),
      last_tx: String::new(),
      owner: String::new(),
      tags: Vec::new(),
      target: String::new(),
      quantity: String::new(),
      data: String::new(),
      reward: String::new(),
      signature: String::new(),
      data_size: String::new(),
      data_root: String::new(),
    },
  }
}
