/**
 *
 * @Purpose: Parse cmd arguments so main.rs has data to execute other libs.
 *           Parse Command Arguments and determine whether cmd is 'Run', 'DryRun' or 'Serve'.
 *           Upon Match, grab the rest of the data from 'pargs' and feed it into the 'Flags' enum.
 *           ParseResult enum which houses the Flags enum is then returned inside Ok()
 *
 * @Note: Code to understand right away
 * `subcommand()` - grabs first argument in cmd i.e. 'Run', 'DryRun' or 'Serve'
 * `opt_value_from_str(key)` - sees if an optional key was provided & extracts value, ex. [--height, 42] -> 42
 * `unwrap_or_else()` - extract value or do something else, ex. Option<String> becomes String
 * `as_deref()` - converts an Option<String> into Option<&str> so we can borrow and not clone any values, more efficient.
 *
 */
use crate::print_help::print_help;
use pico_args::Arguments;
use std::ops::Deref;

#[derive(Debug)]
pub enum Flags {
  Run {
    host: String,
    port: i32,
    protocol: String,
    tx: Option<String>,
    pretty_print: bool,
    no_print: bool,
    show_validity: bool,
    save: bool,
    benchmark: bool,
    save_path: String,
    height: Option<usize>,
    no_cache: bool,
    show_errors: bool,
  },
  DryRun {
    host: String,
    port: i32,
    protocol: String,
    pretty_print: bool,
    show_validity: bool,
    file: Option<String>,
  },
  Serve {
    server_host: String,
    server_port: u16,
  },
}

#[derive(Debug)]
pub enum ParseResult {
  Help { cmd: String },
  Known { flag: Flags },
}

fn parse_node_limit(
  arguments: &mut Arguments,
) -> Result<i32, pico_args::Error> {
  let node_limit = arguments.opt_value_from_str("--node-limit")?.unwrap_or(8);
  if node_limit < 8 {
    panic!("At least 8 nodes are needed.");
  }
  Ok(node_limit)
}

pub fn parse() -> Result<ParseResult, pico_args::Error> {
  let mut pargs = Arguments::from_env(); //Ex. -> Arguments(["arg1", "arg2", "arg3"])
  let is_help = pargs.contains("--help"); //Checks if user entered help flag

  /**
   * subcommand -> Ok(Some("arg1")) grabs first argument
   * as_deref -> Peels back the Ok() wrapper so its Some("arg1")
   * unwrap_or -> Peels back <Some> or panics
   * to_string -> Converts back to mutable string
   */
  let cmd = pargs
    .subcommand()?
    .as_deref()
    .unwrap_or("Unknown")
    .to_string();

  if is_help {
    //Store cmd with --help flag inside Help struct
    Ok(ParseResult::Help {
      cmd: String::from(cmd),
    })
  } else {
    //CHECK IF subcommand was dry-run, run or serve
    #[allow(clippy::wildcard_in_or_patterns)]
    let flags = match cmd.deref() {
      "dry-run" => ParseResult::Known {
        flag: Flags::DryRun {
          host: pargs
            .opt_value_from_str("--host")?
            .unwrap_or_else(|| String::from("arweave.net")),
          port: pargs.opt_value_from_str("--port")?.unwrap_or(80),
          protocol: pargs
            .opt_value_from_str("--protocol")?
            .unwrap_or_else(|| String::from("https")),
          pretty_print: pargs.contains("--pretty-print"),
          show_validity: pargs.contains("--show-validity"),
          file: pargs.opt_value_from_str("--file").unwrap(),
        },
      },
      "run" => ParseResult::Known {
        flag: Flags::Run {
          host: pargs
            .opt_value_from_str("--host")?
            .unwrap_or_else(|| String::from("arweave.net")),
          port: pargs.opt_value_from_str("--port")?.unwrap_or(80),
          protocol: pargs
            .opt_value_from_str("--protocol")?
            .unwrap_or_else(|| String::from("https")),
          tx: pargs.opt_value_from_str("--contract-id").unwrap(),
          pretty_print: pargs.contains("--pretty-print"),
          no_print: pargs.contains("--no-print"),
          show_validity: pargs.contains("--show-validity"),
          save: pargs.contains("--save"),
          benchmark: pargs.contains("--benchmark"),
          save_path: pargs
            .opt_value_from_str("--save")?
            .unwrap_or_else(|| String::from("")),
          height: { pargs.opt_value_from_str("--height").unwrap() },
          no_cache: pargs.contains("--no-cache"),
          show_errors: pargs.contains("--show-errors"),
        },
      },
      "serve" => ParseResult::Known {
        flag: Flags::Serve {
          server_host: pargs
            .opt_value_from_str("--host")?
            .unwrap_or_else(|| String::from("127.0.0.1")),
          server_port: pargs.opt_value_from_str("--port")?.unwrap_or(5400),
        },
      },
      "Unknown" | _ => ParseResult::Help {
        cmd: String::from("none"),
      },
    };

    Ok(flags)
  }
}
