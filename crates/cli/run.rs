use deno_core::error::AnyError;
use std::io::Write;
use three_em_arweave::arweave::Arweave;
use three_em_arweave::cache::ArweaveCache;
use three_em_arweave::cache::CacheExt;
use three_em_executor::execute_contract;
use three_em_executor::executor::ExecuteResult;
use three_em_executor::util::process_execution;

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
  let arweave = Arweave::new(port, host, protocol, ArweaveCache::new());
  let start = std::time::Instant::now();

  let execution =
    execute_contract(&arweave, tx, None, None, height, !no_cache, show_errors)
      .await?;

  if benchmark {
    let elapsed = start.elapsed();
    println!("Took {}ms to execute contract", elapsed.as_millis());
  }

  let process_execution_val = process_execution(execution, show_validity);

  if !no_print {
    if pretty_print {
      println!(
        "{}",
        serde_json::to_string_pretty(&process_execution_val).unwrap()
      );
    } else {
      println!("{}", process_execution_val);
    }
  }

  if save {
    let mut file = std::fs::File::create(save_path).unwrap();
    file
      .write_all(
        serde_json::to_vec(&process_execution_val)
          .unwrap()
          .as_slice(),
      )
      .unwrap();
  }

  Ok(())
}
