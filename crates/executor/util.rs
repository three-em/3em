use crate::ExecuteResult;
use deno_core::serde_json::Value;

pub fn process_execution(
  execute_result: ExecuteResult,
  show_validity: bool,
) -> Value {
  match execute_result {
    ExecuteResult::V8(value, validity_table) => {
      if show_validity {
        serde_json::json!({
            "state": value,
            "validity": validity_table
        })
      } else {
        value
      }
    }
    ExecuteResult::Evm(store, result, validity_table) => {
      let store = hex::encode(store.raw());
      let result = hex::encode(result);

      if show_validity {
        serde_json::json!({
            "result": result,
            "store": store,
            "validity": validity_table
        })
      } else {
        serde_json::json!({
            "result": result,
            "store": store,
        })
      }
    }
  }
}
