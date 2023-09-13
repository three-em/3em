use deno_core::error::AnyError;
use std::io::Write;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_executor::execute_contract;
use three_em_executor::executor::ExecuteResult;

#[allow(clippy::too_many_arguments)]
pub async fn run(
  port: i32,
  host: String,
  protocol: String,
  tx: String,
  pretty_print: bool,
  no_print: bool,
  show_validity: bool,
  save: bool,
  benchmark: bool,
  save_path: String,
  height: Option<usize>,
  no_cache: bool,
  show_errors: bool,
) -> Result<(), AnyError> {
  // Create a new Arweave Object with a new cache
  let arweave = Arweave::new(port, host, protocol, ArweaveCache::new());
  let start = std::time::Instant::now();

  //Run contract based on contract id - this is only a runtime so no input is sent here
  let execution: ExecuteResult =
    execute_contract(tx, height, !no_cache, show_errors, None, None, &arweave)
      .await?;

  if benchmark {
    let elapsed = start.elapsed();
    println!("Took {}ms to execute contract", elapsed.as_millis());
  }

  match execution {
    ExecuteResult::V8(data) => {
      let state = data.state;
      let validity_table = data.validity;
      let result = data.result.unwrap_or(serde_json::Value::Null);
      // return state and result when endpoint is reached.
      let value = if show_validity {
        serde_json::json!({
            "state": state,
            "validity": validity_table,
            "result": result
        })
      } else {
        state
      };

      if !no_print {
        if pretty_print {
          println!("{}", serde_json::to_string_pretty(&value).unwrap());
        } else {
          println!("{}", value);
        }
      }

      if save {
        let mut file = std::fs::File::create(save_path).unwrap();
        file
          .write_all(serde_json::to_vec(&value).unwrap().as_slice())
          .unwrap();
      }
    }
    ExecuteResult::Evm(store, result, validity_table) => {
      let store = hex::encode(store.raw());
      let result = hex::encode(result);

      let value = if show_validity {
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
      };

      if !no_print {
        if pretty_print {
          println!("{}", serde_json::to_string_pretty(&value).unwrap());
        } else {
          println!("{}", value);
        }
      }
    }
  }

  Ok(())
}
