mod cli;
mod core_nodes;
mod messages;
mod node;
mod node_crypto;
mod runtime;
mod start;
mod utils;

use cli::parse::Flags;
use deno_core::error::AnyError;

use colored::Colorize;
use std::{env, thread};

static BANNER: &str = r#"
██████╗     ███████╗    ███╗   ███╗
╚════██╗    ██╔════╝    ████╗ ████║
 █████╔╝    █████╗      ██╔████╔██║
 ╚═══██╗    ██╔══╝      ██║╚██╔╝██║
██████╔╝    ███████╗    ██║ ╚═╝ ██║
╚═════╝     ╚══════╝    ╚═╝     ╚═╝

The Web3 Execution Machine
Languages supported: Javascript, Rust, C++, C, C#.
"#;

static USAGE: &str = r#"
    USAGE:
      three_em [OPTIONS] [SUBCOMMAND]

    For more information try --help
"#;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
  println!("{}", BANNER);
  println!("Version: {}", env!("CARGO_PKG_VERSION"));
  println!();

  let flags = cli::parse::parse()?;

  match flags {
    Flags::Start {
      host,
      port,
      node_capacity,
    } => {
      start::start(host, port, node_capacity).await?;
    }
    Flags::Unknown(cmd) => {
      print_cmd_error(&cmd);
    }
    Flags::Run {
      arweave_port,
      arweave_host,
      arweave_protocol,
      contract_id,
    } => {
      let arweave =
        runtime::core::arweave::Arweave::new(arweave_port, arweave_host);
      runtime::core::execute::execute_contract(
        &arweave,
        contract_id,
        None,
        None,
        None,
      )
      .await;
    }
  };

  Ok(())
}

fn print_cmd_error(cmd: &String) {
  println!("{}: Found argument '{}' which wasn't expected, or isn't valid in this context.", "error".red(), cmd);
  println!("{}", USAGE);
}
