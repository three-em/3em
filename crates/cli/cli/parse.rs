use pico_args::Arguments;

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
    tx: String,
    pretty_print: bool,
    no_print: bool,
    show_validity: bool,
    save: bool,
    benchmark: bool,
    save_path: String,
    height: Option<usize>,
    no_cache: bool,
  },
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

pub fn parse() -> Result<Flags, pico_args::Error> {
  let mut pargs = Arguments::from_env();

  #[allow(clippy::wildcard_in_or_patterns)]
  let flags = match pargs.subcommand()?.as_deref().unwrap_or("Unknown") {
    "start" => Flags::Start {
      port: pargs.opt_value_from_str("--port")?.unwrap_or(8755),
      host: pargs
        .opt_value_from_str("--host")?
        .unwrap_or_else(|| String::from("127.0.0.1")),
      node_capacity: parse_node_limit(&mut pargs).unwrap(),
    },
    "run" | _ => Flags::Run {
      host: pargs
        .opt_value_from_str("--arweave-host")?
        .unwrap_or_else(|| String::from("arweave.net")),
      port: pargs.opt_value_from_str("--arweave-port")?.unwrap_or(80),
      protocol: pargs
        .opt_value_from_str("--arweave-protocol")?
        .unwrap_or_else(|| String::from("https")),
      tx: pargs
        .opt_value_from_str("--contract-id")?
        .unwrap_or_else(|| {
          String::from("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE")
        }),
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
    },
  };

  Ok(flags)
}
