use crate::runtime::core::arweave_get_tag::get_tag;
use crate::runtime::core::gql_result::{
  GQLEdgeInterface, GQLNodeParent, GQLTransactionsResultInterface,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct NetworkInfo {
  pub network: String,
  pub version: usize,
  pub release: usize,
  pub height: usize,
  pub current: String,
  pub blocks: usize,
  pub peers: usize,
  pub queue_length: usize,
  pub node_state_latency: usize,
}

#[derive(Deserialize, Debug)]
pub struct Tag {
  pub name: String,
  pub value: String,
}

#[derive(Deserialize, Debug)]
pub struct TransactionData {
  pub format: usize,
  pub id: String,
  pub last_tx: String,
  pub owner: String,
  pub tags: Vec<Tag>,
  pub target: String,
  pub quantity: String,
  pub data: Vec<u8>,
  pub reward: String,
  pub signature: String,
  pub data_size: String,
  pub data_root: String,
}

pub enum ArweaveProtocol {
  HTTP,
  HTTPS,
}

pub struct Arweave {
  host: String,
  port: i32,
  protocol: ArweaveProtocol,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct TagFilter {
  name: String,
  values: Vec<String>,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct BlockFilter {
  max: usize,
}

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct InteractionVariables {
  tags: Vec<TagFilter>,
  block_filter: BlockFilter,
  first: usize,
  #[serde(skip_serializing_if = "Option::is_none")]
  #[serde(default)]
  after: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct GraphqlQuery {
  query: String,
  variables: InteractionVariables,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct LoadedContract {
  id: String,
  contract_src_tx_id: String,
  contract_src: Vec<u8>,
  contract_type: String, // application/javascript , application/solidity, application/wasm
  init_state: String,
  min_fee: String,
  contract_transaction: TransactionData,
}

pub static MAX_REQUEST: &'static i32 = &100;

impl Arweave {
  pub fn new(port: i32, host: String) -> Arweave {
    return Arweave {
      port,
      host,
      protocol: ArweaveProtocol::HTTPS,
    };
  }

  pub fn get_transaction(
    &self,
    transaction_id: String,
  ) -> reqwest::Result<TransactionData> {
    let transaction = reqwest::blocking::get(format!(
      "{}/tx/{}",
      self.get_host(),
      transaction_id
    ))
    .unwrap()
    .json::<TransactionData>();
    transaction
  }

  pub fn get_network_info(&self) -> NetworkInfo {
    let info = reqwest::blocking::get(format!("{}/info", self.get_host()))
      .unwrap()
      .json::<NetworkInfo>()
      .unwrap();
    info
  }

  pub fn get_interactions(
    &self,
    contract_id: String,
    height: Option<usize>,
  ) -> Vec<GQLEdgeInterface> {
    let variables =
      self.get_default_gql_variables(contract_id.to_owned(), height.to_owned());

    let mut transactions = self.get_next_interaction_page(variables.clone());

    let mut tx_infos = transactions.edges.clone();

    while transactions.page_info.has_next_page {
      let edge = transactions
        .edges
        .get((self.get_max_requests() - 1) as usize)
        .unwrap();
      let cursor = (&edge.cursor).to_owned();

      let variables = self
        .get_default_gql_variables(contract_id.to_owned(), height.to_owned());
      let mut new_variables: InteractionVariables = variables.clone();
      new_variables.after = Some(cursor);

      transactions = self.get_next_interaction_page(new_variables);
      tx_infos.extend(transactions.edges.to_owned());
    }

    let filtered: Vec<GQLEdgeInterface> = tx_infos
      .into_iter()
      .filter(|p| {
        (p.node.parent.is_none())
          || (p
            .node
            .parent
            .as_ref()
            .unwrap_or_else(|| &GQLNodeParent { id: None }))
          .id
          .is_none()
      })
      .collect();

    filtered
  }

  fn get_next_interaction_page(
    &self,
    variables: InteractionVariables,
  ) -> GQLTransactionsResultInterface {
    let query = r#"query Transactions($tags: [TagFilter!]!, $blockFilter: BlockFilter!, $first: Int!, $after: String) {
    transactions(tags: $tags, block: $blockFilter, first: $first, sort: HEIGHT_ASC, after: $after) {
      pageInfo {
        hasNextPage
      }
      edges {
        node {
          id
          owner { address }
          recipient
          tags {
            name
            value
          }
          block {
            height
            id
            timestamp
          }
          fee { winston }
          quantity { winston }
          parent { id }
        }
        cursor
      }
    }
  }"#;

    let query_str = String::from(query);

    let graphql_query = GraphqlQuery {
      query: query_str,
      variables,
    };

    let req_url = format!("{}/graphql", self.get_host());
    let result = reqwest::blocking::Client::new()
      .post(req_url)
      .json(&graphql_query)
      .send()
      .unwrap();

    let data = result.json::<GQLTransactionsResultInterface>().unwrap();

    data
  }

  pub fn load_contract(
    &self,
    contract_id: String,
    contract_src_tx_id: Option<String>,
  ) -> LoadedContract {
    // TODO: DON'T PANNIC
    let contract_transaction =
      self.get_transaction(contract_id.to_owned()).unwrap();

    let contract_src = contract_src_tx_id
      .unwrap_or_else(|| get_tag(&contract_transaction, "Contract-Src"));

    if contract_src.len() <= 0 {
      panic!("Contract contains invalid information for tag 'Contract-Src'");
    }

    let min_fee = get_tag(&contract_transaction, "Min-Fee");

    // TODO: Don't panic
    let contract_src_tx =
      self.get_transaction(contract_src.to_owned()).unwrap();

    let contract_src_data = contract_src_tx.data.to_owned();

    let mut state = String::from("");

    let init_state_tag = get_tag(&contract_transaction, "Init-State");
    if init_state_tag.len() >= 1 {
      state = init_state_tag;
    } else {
      let init_state_tag_txid = get_tag(&contract_transaction, "Init-State-TX");
      if init_state_tag_txid.len() >= 1 {
        let init_state_tx = self.get_transaction(init_state_tag_txid).unwrap();
        state = String::from_utf8(init_state_tx.data).unwrap();
      } else {
        state =
          String::from_utf8(contract_transaction.data.to_owned()).unwrap();
      }
    }

    LoadedContract {
      id: contract_id,
      contract_src_tx_id: contract_src,
      contract_src: contract_src_data,
      contract_type: String::from("application/javascript"), // TODO: Handle wasm, evm, etc.
      init_state: state,
      min_fee,
      contract_transaction,
    }
  }

  fn get_host(&self) -> String {
    let protocol = match self.protocol {
      ArweaveProtocol::HTTP => "http",
      ArweaveProtocol::HTTPS => "https",
    };

    if self.port == 80 {
      format!("{}://{}", protocol, self.host)
    } else {
      format!("{}://{}:{}", protocol, self.host, self.port)
    }
  }

  fn get_default_gql_variables(
    &self,
    contract_id: String,
    height: Option<usize>,
  ) -> InteractionVariables {
    let app_name_tag: TagFilter = TagFilter {
      name: "App-Name".to_owned(),
      values: vec!["SmartWeaveAction".to_owned()],
    };

    let contract_tag: TagFilter = TagFilter {
      name: "Contract".to_owned(),
      values: vec![contract_id],
    };

    let new_height = match height {
      Some(size) => size,
      None => self.get_network_info().height,
    };

    let variables: InteractionVariables = InteractionVariables {
      tags: vec![app_name_tag, contract_tag],
      block_filter: BlockFilter { max: new_height },
      first: self.get_max_requests() as usize,
      after: None,
    };

    variables
  }

  fn get_max_requests(&self) -> i32 {
    MAX_REQUEST.to_owned()
  }
}
