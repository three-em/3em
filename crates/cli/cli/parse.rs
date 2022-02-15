use crate::print_help::print_help;
use pico_args::Arguments;
use std::ops::Deref;

#[derive(Debug)]
pub enum Flags {
  Start {
    port: i32,
    host: String,
    node_capacity: i32,
  },
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
    meaningful_error: bool,
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
  let mut pargs = Arguments::from_env();
  let is_help = pargs.contains("--help");

  let cmd = pargs
    .subcommand()?
    .as_deref()
    .unwrap_or("Unknown")
    .to_string();

  if is_help {
    Ok(ParseResult::Help {
      cmd: String::from(cmd),
    })
  } else {
    #[allow(clippy::wildcard_in_or_patterns)]
    let flags = match cmd.deref() {
      "start" => ParseResult::Known {
        flag: Flags::Start {
          port: pargs.opt_value_from_str("--port")?.unwrap_or(8755),
          host: pargs
            .opt_value_from_str("--host")?
            .unwrap_or_else(|| String::from("127.0.0.1")),
          node_capacity: parse_node_limit(&mut pargs).unwrap(),
        },
      },
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
          meaningful_error: pargs.contains("--meaningful-error"),
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
