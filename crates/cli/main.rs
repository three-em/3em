mod cli;
mod core_nodes;
mod dry_run;
mod local_server;
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

use crate::local_server::{start_local_server, ServerConfiguration};
use std::env;
use std::net::IpAddr;
use std::ops::Deref;
use std::str::FromStr;

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

fn main() -> Result<(), AnyError> {
  println!("{}", BANNER);
  println!("Version: {}", env!("CARGO_PKG_VERSION"));
  println!();

  let parse_result = parse::parse()?;

  let rt = tokio::runtime::Runtime::new()?;

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
          rt.block_on(crate::start::start(host, port, node_capacity))?;
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
            rt.block_on(run::run(
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
            ))?;
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
            rt.block_on(dry_run::dry_run(
              port,
              host,
              protocol,
              pretty_print,
              show_validity,
              file.unwrap(),
            ))?;
          }
        }
        Flags::Serve {
          server_port,
          server_host,
        } => {
          let ip_addr = IpAddr::from_str(server_host.as_str());
          if let Err(_) = ip_addr {
            print_help::print_help(Some("serve"));
            println!("{}", "Invalid IP Address provided in '--server-host'");
          } else {
            // Spawn the !Send future in the currently running
            // local task set.
            let local = tokio::task::LocalSet::new();
            local.block_on(
              &rt,
              start_local_server(ServerConfiguration {
                host: ip_addr.unwrap(),
                port: server_port,
              }),
            );
          }
        }
      };
    }
  }

  Ok(())
}
