mod cli;
mod core_nodes;
mod dry_run;
mod messages;
mod node;
mod node_crypto;
mod run;
mod start;
mod utils;

use crate::cli::parse;
use crate::cli::parse::Flags;
use deno_core::error::AnyError;

use std::env;

static BANNER: &str = r#"
██████╗     ███████╗    ███╗   ███╗
╚════██╗    ██╔════╝    ████╗ ████║
 █████╔╝    █████╗      ██╔████╔██║
 ╚═══██╗    ██╔══╝      ██║╚██╔╝██║
██████╔╝    ███████╗    ██║ ╚═╝ ██║
╚═════╝     ╚══════╝    ╚═╝     ╚═╝

The Web3 Execution Machine
Languages supported: Javascript, Solidity, Rust, C++, C, AssemblyScript, Zig, Vyper.
"#;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
  println!("{}", BANNER);
  println!("Version: {}", env!("CARGO_PKG_VERSION"));
  println!();

  let flags = parse::parse()?;

  match flags {
    Flags::Start {
      host,
      port,
      node_capacity,
    } => {
      crate::start::start(host, port, node_capacity).await?;
    }
    Flags::Run {
      port,
      host,
      protocol,
      tx,
      pretty_print,
      no_print,
      show_validity,
      save,
      save_path,
      benchmark,
      height,
      no_cache,
    } => {
      run::run(
        port,
        host,
        protocol,
        tx,
        pretty_print,
        no_print,
        show_validity,
        save,
        benchmark,
        save_path,
        height,
        no_cache,
      )
      .await?;
    }
    Flags::DryRun {
      host,
      port,
      protocol,
      pretty_print,
      show_validity,
      file,
    } => {
      dry_run::dry_run(port, host, protocol, pretty_print, show_validity, file)
        .await?;
    }
  };

  Ok(())
}
