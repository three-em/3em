use crate::runtime::core::arweave::Arweave;
use crate::runtime::core::execute::{execute_contract, ExecuteResult};
use std::io::Write;

pub async fn run(
  port: i32,
  host: String,
  tx: String,
  pretty_print: bool,
  no_print: bool,
  show_validity: bool,
  save: bool,
  benchmark: bool,
  save_path: String,
  height: Option<usize>
) {
  let arweave = Arweave::new(port, host);
  let start = std::time::Instant::now();

  let execution = execute_contract(arweave, tx, None, None, height).await;

  if benchmark {
    let elapsed = start.elapsed();
    println!("Took {}ms to execute contract", elapsed.as_millis());
  }

  match execution {
    ExecuteResult::V8(value, validity_table) => {
      let value = if show_validity {
        serde_json::json!({
            "state": value,
            "validity": validity_table
        })
      } else {
        value
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
        file.write_all(serde_json::to_vec(&value).unwrap().as_slice());
      }
    }
    ExecuteResult::Evm(value, validity_table) => {}
  }
}
