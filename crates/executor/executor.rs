use crate::{get_input_from_interaction, nop_cost_fn};
use deno_core::serde_json;
use deno_core::serde_json::Value;
use indexmap::map::IndexMap;
use std::collections::HashMap;
use std::env;
use three_em_arweave::arweave::LoadedContract;
use three_em_arweave::arweave::ARWEAVE_CACHE;
use three_em_arweave::arweave::{Arweave, ArweaveProtocol};
use three_em_arweave::gql_result::{
  GQLAmountInterface, GQLEdgeInterface, GQLNodeInterface,
};
use three_em_arweave::miscellaneous::ContractType;
use three_em_evm::{ExecutionState, Machine, Storage};
use three_em_js::CallResult;
use three_em_js::Runtime;
use three_em_smartweave::{ContractBlock, ContractInfo};
use three_em_wasm::WasmRuntime;

pub type ValidityTable = IndexMap<String, bool>;
pub type CachedState = Option<Value>;

pub enum ExecuteResult {
  V8(Value, ValidityTable),
  Evm(Storage, Vec<u8>, ValidityTable),
}

pub type OnCached = dyn Fn() -> ExecuteResult;

#[allow(clippy::too_many_arguments)]
pub async fn raw_execute_contract<
  CachedCallBack: FnOnce(ValidityTable, CachedState) -> ExecuteResult,
>(
  contract_id: String,
  loaded_contract: LoadedContract,
  interactions: Vec<GQLEdgeInterface>,
  mut validity: IndexMap<String, bool>,
  cache_state: Option<Value>,
  needs_processing: bool,
  show_errors: bool,
  on_cached: CachedCallBack,
  shared_client: Arweave,
) -> ExecuteResult {
  let transaction = (&loaded_contract.contract_transaction).to_owned();
  let cache = cache_state.is_some();
  let arweave_info = (
    shared_client.port.to_owned(),
    shared_client.host.to_owned(),
    match shared_client.protocol.to_owned() {
      ArweaveProtocol::HTTPS => String::from("https"),
      ArweaveProtocol::HTTP => String::from("http"),
    },
  );

  match loaded_contract.contract_type {
    ContractType::JAVASCRIPT => {
      if needs_processing {
        let state: Value = cache_state.unwrap_or_else(|| {
          deno_core::serde_json::from_str(&loaded_contract.init_state).unwrap()
        });

        let mut rt = Runtime::new(
          &(String::from_utf8(loaded_contract.contract_src).unwrap()),
          state,
          arweave_info.to_owned(),
        )
        .await
        .unwrap();

        for interaction in interactions {
          let tx = interaction.node;

          let input = get_input_from_interaction(&tx);

          // TODO: has_multiple_interactions
          // https://github.com/ArweaveTeam/SmartWeave/blob/4d09c66d832091805f583ba73e8da96cde2c0190/src/contract-read.ts#L68
          let js_input: Value = deno_core::serde_json::from_str(input).unwrap();

          let call_input = serde_json::json!({
            "input": js_input,
            "caller": tx.owner.address
          });

          let interaction_context = serde_json::json!({
                    "transaction": {
                      "id": &tx.id,
                      "owner": &tx.owner.address,
                      "target": &tx.recipient.to_owned().unwrap_or_else(|| String::from("null")),
                      "quantity": &tx.quantity.to_owned().unwrap_or_else(|| GQLAmountInterface {
            winston: Some(String::new()),
            ar: Some(String::new())
          }).winston.unwrap(),
                      "reward": &tx.fee.to_owned().unwrap_or_else(|| GQLAmountInterface {
            winston: Some(String::new()),
            ar: Some(String::new())
          }).winston.unwrap(),
                      "tags": &tx.tags
                    },
                    "block": {
                      "indep_hash":  &tx.block.id,
                      "height": &tx.block.height,
                      "timestamp": &tx.block.timestamp
                    }
                  });

          let valid = match rt.call(call_input, Some(interaction_context)).await
          {
            Ok(None) => true,
            Ok(Some(CallResult::Evolve(evolve))) => {
              let contract = shared_client
                .load_contract(contract_id.clone(), Some(evolve), None, true)
                .await
                .unwrap();

              let state: Value = rt.get_contract_state().unwrap();
              rt = Runtime::new(
                &(String::from_utf8_lossy(&contract.contract_src)),
                state,
                arweave_info.to_owned(),
              )
              .await
              .unwrap();

              true
            }
            Ok(Some(CallResult::Result(_))) => true,
            Err(err) => {
              if show_errors {
                println!("{}", err);
              }
              false
            }
          };

          validity.insert(tx.id, valid);
        }

        let state_val: Value = rt.get_contract_state().unwrap();

        if cache {
          ARWEAVE_CACHE
            .cache_states(contract_id, &state_val, &validity)
            .await;
        }

        ExecuteResult::V8(state_val, validity)
      } else {
        on_cached(validity, cache_state)
      }
    }
    ContractType::WASM => {
      let contract_info = ContractInfo {
        transaction,
        block: ContractBlock {
          height: 0,
          indep_hash: String::from(""),
          timestamp: String::from(""),
        },
      };

      if needs_processing {
        let wasm = loaded_contract.contract_src.as_slice();

        let init_state_wasm = if cache_state.is_some() {
          let cache_state_unwrapped = cache_state.unwrap();
          let state_str = cache_state_unwrapped.to_string();
          state_str.as_bytes().to_vec()
        } else {
          loaded_contract.init_state.as_bytes().to_vec()
        };

        let mut state = init_state_wasm;
        let mut rt = WasmRuntime::new(wasm, contract_info).unwrap();

        for interaction in interactions {
          let tx = interaction.node;

          let input = get_input_from_interaction(&tx);
          let wasm_input: Value =
            deno_core::serde_json::from_str(input).unwrap();
          let call_input = serde_json::json!({
            "input": wasm_input,
            "caller": tx.owner.address,
          });

          let mut input = deno_core::serde_json::to_vec(&call_input).unwrap();
          let exec = rt.call(&mut state, &mut input);
          let valid_with_result = match exec {
            Ok(result) => (true, Some(result)),
            Err(err) => {
              if show_errors {
                println!("{}", err);
              }
              (false, None)
            }
          };
          let valid = valid_with_result.0;

          if valid {
            state = valid_with_result.1.unwrap();
          }
          validity.insert(tx.id, valid);
        }

        let state: Value = deno_core::serde_json::from_slice(&state).unwrap();

        if cache {
          ARWEAVE_CACHE
            .cache_states(contract_id, &state, &validity)
            .await;
        }

        ExecuteResult::V8(state, validity)
      } else {
        on_cached(validity, cache_state)
      }
    }
    ContractType::EVM => {
      // Contract source bytes.
      let bytecode = hex::decode(loaded_contract.contract_src.as_slice())
        .expect("Failed to decode contract bytecode");
      let store = hex::decode(loaded_contract.init_state.as_bytes())
        .expect("Failed to decode account state");

      let mut account_store = Storage::from_raw(&store);
      let mut result = vec![];
      for interaction in interactions {
        let tx = interaction.node;
        let block_info =
          shared_client.get_transaction_block(&tx.id).await.unwrap();

        let block_info = three_em_evm::BlockInfo {
          timestamp: three_em_evm::U256::from(block_info.timestamp),
          difficulty: three_em_evm::U256::from_str_radix(&block_info.diff, 10)
            .unwrap(),
          block_hash: three_em_evm::U256::from(
            block_info.indep_hash.as_bytes(),
          ),
          number: three_em_evm::U256::from(block_info.height),
        };

        let input = get_input_from_interaction(&tx);
        let call_data = hex::decode(input).expect("Failed to decode input");

        let mut machine = Machine::new_with_data(nop_cost_fn, call_data);
        machine.set_storage(account_store.clone());

        machine.set_fetcher(Box::new(|address: &three_em_evm::U256| {
          let mut id = [0u8; 32];
          address.to_big_endian(&mut id);
          let id = String::from_utf8_lossy(&id).to_string();
          let contract = deno_core::futures::executor::block_on(
            shared_client.load_contract(id, None, None, cache),
          )
          .expect("evm call: Failed to load contract");

          let bytecode = hex::decode(contract.contract_src.as_slice())
            .expect("Failed to decode contract bytecode");
          let store = hex::decode(contract.init_state.as_bytes())
            .expect("Failed to decode account state");

          let store = Storage::from_raw(&store);

          Some(three_em_evm::ContractInfo { store, bytecode })
        }));

        match machine.execute(&bytecode, block_info) {
          ExecutionState::Abort(_) | ExecutionState::Revert => {
            validity.insert(tx.id, false);
          }
          ExecutionState::Ok => {
            account_store = machine.storage;
            result = machine.result;
            validity.insert(tx.id, true);
          }
        }
      }

      ExecuteResult::Evm(account_store, result, validity)
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::executor::{raw_execute_contract, ExecuteResult};
  use crate::test_util::{
    generate_fake_interaction, generate_fake_loaded_contract_data,
  };
  use deno_core::serde_json;
  use deno_core::serde_json::Value;
  use indexmap::map::IndexMap;
  use three_em_arweave::arweave::Arweave;
  use three_em_arweave::arweave::{LoadedContract, TransactionData};
  use three_em_arweave::gql_result::{
    GQLAmountInterface, GQLBlockInterface, GQLEdgeInterface, GQLNodeInterface,
    GQLOwnerInterface, GQLTagInterface,
  };
  use three_em_arweave::miscellaneous::ContractType;

  #[tokio::test]
  async fn test_globals_js() {
    let init_state = serde_json::json!({});

    let fake_contract = generate_fake_loaded_contract_data(
      include_bytes!("../../testdata/contracts/globals_contract.js"),
      ContractType::JAVASCRIPT,
      init_state.to_string(),
    );

    let mut transaction1 = generate_fake_interaction(
      serde_json::json!({}),
      "tx1123123123123123123213213123",
      Some(String::from("ABCD-EFG")),
      Some(2),
      Some(String::from("SUPERMAN1293120")),
      Some(String::from("RECIPIENT1234")),
      Some(GQLTagInterface {
        name: String::from("MyTag"),
        value: String::from("Christpoher Nolan is awesome"),
      }),
      Some(GQLAmountInterface {
        winston: Some(String::from("100")),
        ar: None,
      }),
      Some(GQLAmountInterface {
        winston: Some(String::from("100")),
        ar: None,
      }),
      Some(12301239),
    );

    let fake_interactions = vec![transaction1];

    let result = raw_execute_contract(
      String::from("10230123021302130"),
      fake_contract,
      fake_interactions,
      IndexMap::new(),
      None,
      true,
      false,
      |_, _| {
        panic!("not implemented");
      },
      Arweave::new(443, "arweave.net".to_string(), String::from("https")),
    )
    .await;

    if let ExecuteResult::V8(value, validity) = result {
      assert_eq!(
        value,
        serde_json::json!({"txId":"tx1123123123123123123213213123","txOwner":"SUPERMAN1293120","txTarget":"RECIPIENT1234","txQuantity":"100","txReward":"100","txTags":[{"name":"Input","value":"{}"},{"name":"MyTag","value":"Christpoher Nolan is awesome"}],"txHeight":2,"txIndepHash":"ABCD-EFG","txTimestamp":12301239,"winstonToAr":true,"arToWinston":true,"compareArWinston":1})
      );
    } else {
      panic!("Unexpected entry");
    }
  }

  #[tokio::test]
  pub async fn test_executor_js() {
    let init_state = serde_json::json!({
      "users": []
    });

    let fake_contract = generate_fake_loaded_contract_data(
      include_bytes!("../../testdata/contracts/users_contract.js"),
      ContractType::JAVASCRIPT,
      init_state.to_string(),
    );
    let fake_interactions = vec![
      generate_fake_interaction(
        serde_json::json!({
          "function": "add",
          "name": "Andres"
        }),
        "tx1",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
      ),
      generate_fake_interaction(
        serde_json::json!({
          "function": "none",
          "name": "Tate"
        }),
        "tx2",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
      ),
      generate_fake_interaction(
        serde_json::json!({
          "function": "add",
          "name": "Divy"
        }),
        "tx3",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
      ),
    ];

    let result = raw_execute_contract(
      String::new(),
      fake_contract,
      fake_interactions,
      IndexMap::new(),
      None,
      true,
      false,
      |_, _| {
        panic!("not implemented");
      },
      Arweave::new(443, "arweave.net".to_string(), String::from("https")),
    )
    .await;

    if let ExecuteResult::V8(value, validity) = result {
      assert_eq!(validity.len(), 3);
      let (tx1, tx2, tx3) = (
        validity.get("tx1"),
        validity.get("tx2"),
        validity.get("tx3"),
      );
      assert_eq!(tx1.is_some(), true);
      assert_eq!(tx2.is_some(), true);
      assert_eq!(tx3.is_some(), true);
      assert_eq!((tx1.unwrap()).to_owned(), true);
      assert_eq!((tx2.unwrap()).to_owned(), false);
      assert_eq!((tx3.unwrap()).to_owned(), true);

      let value_state = value.get("users");
      assert_eq!(value_state.is_some(), true);
      let users = value_state
        .unwrap()
        .to_owned()
        .as_array()
        .unwrap()
        .to_owned();
      assert_eq!(
        users.get(0).unwrap().to_owned(),
        serde_json::json!("Andres")
      );
      assert_eq!(users.get(1).unwrap().to_owned(), serde_json::json!("Divy"));
    } else {
      panic!("Failed");
    }
  }

  #[tokio::test]
  async fn test_contract_evolve() {
    let init_state = serde_json::json!({
      "canEvolve": true,
      "v": 0,
      "i": 0,
      "pi": 0
    });

    let fake_contract = generate_fake_loaded_contract_data(
      include_bytes!("../../testdata/evolve/evolve1.js"),
      ContractType::JAVASCRIPT,
      init_state.to_string(),
    );

    let fake_interactions = vec![
      generate_fake_interaction(
        serde_json::json!({
          "function": "evolve",
          // Contract Source for "Zwp7r7Z10O0TuF6lmFApB7m5lJIrE5RbLAVWg_WKNcU"
          "value": "C0F9QvOOJNR2DDIicWeL9B-C5vFrtczmOjpW_3FCQBQ",
        }),
        "tx1",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
      ),
      generate_fake_interaction(
        serde_json::json!({
          "function": "contribute",
        }),
        "tx2",
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
      ),
    ];

    let result = raw_execute_contract(
      String::from("Zwp7r7Z10O0TuF6lmFApB7m5lJIrE5RbLAVWg_WKNcU"),
      fake_contract,
      fake_interactions,
      IndexMap::new(),
      None,
      true,
      false,
      |_, _| {
        panic!("not implemented");
      },
      Arweave::new(443, "arweave.net".to_string(), String::from("https")),
    )
    .await;

    if let ExecuteResult::V8(value, validity) = result {
      // tx1 is the evolve action. This must not fail.
      // All network calls happen here and runtime is
      // re-initialized.
      assert_eq!(validity.get("tx1").unwrap(), &true);
      // tx2 is the interaction to the evolved source.
      // In this case, the pi contract.
      assert_eq!(validity.get("tx2").unwrap(), &true);

      let value_state = value.get("canEvolve");
      assert_eq!(value_state.is_some(), true);

      assert_eq!(value.get("v").unwrap(), 0.6666666666666667_f64);
    }
  }
}
