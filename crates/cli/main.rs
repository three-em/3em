mod cli;
mod core_nodes;
mod dry_run;
mod messages;
mod node;
mod node_crypto;
mod print_help;
mod run;
mod start;
mod utils;

use crate::cli::parse;
use crate::cli::parse::{Flags, ParseResult};
use deno_core::error::AnyError;

use std::env;
use std::ops::Deref;

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

  let parse_result = parse::parse()?;

  match parse_result {
    ParseResult::Help { cmd } => {
      print_help::print_help(Some(cmd.deref()));
    }
    ParseResult::Known { flag } => {
      match flag {
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
          show_errors,
        } => {
          if tx.is_none() {
            print_help::print_help(Some("run"));
            println!("{}", "Option '--contract-id' is required");
          } else {
            run::run(
              port,
              host,
              protocol,
              tx.unwrap(),
              pretty_print,
              no_print,
              show_validity,
              save,
              benchmark,
              save_path,
              height,
              no_cache,
              show_errors,
            )
            .await?;
          }
        }
        Flags::DryRun {
          host,
          port,
          protocol,
          pretty_print,
          show_validity,
          file,
        } => {
          if file.is_none() {
            print_help::print_help(Some("dry-run"));
            println!("{}", "Option '--file' is required");
          } else {
            dry_run::dry_run(
              port,
              host,
              protocol,
              pretty_print,
              show_validity,
              file.unwrap(),
            )
            .await?;
          }
        }
      };
    }
  }

  Ok(())
}
