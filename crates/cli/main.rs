/**
 * @Purpose
 *          Parses cmd arguments.
 *          Initiates a runtime to manage concurrency and schedule tasks
 * 
 * 
 */

// Imports the moduls in the sister files next to main
mod cli;
mod core_nodes;
mod dry_run;
mod local_server;
mod messages;
mod node;
mod print_help;
mod run;
mod utils;

use crate::cli::parse;
use crate::cli::parse::{Flags, ParseResult};
use deno_core::error::AnyError;

use crate::local_server::{start_local_server, ServerConfiguration};
use std::env;
use std::net::IpAddr;
use std::ops::Deref;
use std::str::FromStr;

fn main() -> Result<(), AnyError> {
  let parse_result = parse::parse()?;

  /**
   * @Runtime 
   * Will Manage Concurrent Tasks with Ease 
   * Such as async operations, start/pause/schedule tasks
   * 
   * What is a runtime?
   * A layer on top of the OS. 
   * Executable files are placed into RAM, the processor turns the executable into machine code and runs the program.
   * The lifetime execution of the program is the runtime. 
   * 
   * Why use Runtime?
   * Rust has built in async/await and thread operations
   * But more complex ops such as task scheduling, thread pools, or asynchronous I/O
   *    need an external system to manage this for us. 
   * Building a thread pool from scratch would require use to manually make 
   *    a dedicated Worker struct that saves JoinHandle types responsible for tracking a thread.
   *    And of course there are tons of other features we would have to build that tokio automates. 
   * 
   * @Note Functions to learn
   * `block_on` - entry point into the runtime, WILL BLOCK main thread - need to see why this was picked. 
   * `future` - a future task to be completed
   * `DryRun` - testing the programming logic without starting the program
   * 
   * @Note Flags::Run is the entry point to execute an instance of the contract ready for IO-bound operations
   * 
   */
  let rt = tokio::runtime::Runtime::new()?;

  match parse_result {
    ParseResult::Help { cmd } => {
      print_help::print_help(Some(cmd.deref())); //Prints message containing list of cmds, options etc. 
    }

    ParseResult::Known { flag } => { // Match whether user wants Run, DryRun, Serve
      match flag {
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
            //run a new Arweave object w/ cache established - blocking the thread until future task complete
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
        Flags::DryRun { // Used to test code
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
        Flags::Serve { //Spins up a local testnet
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
