use pico_args::Arguments;

#[derive(Debug)]
pub enum Flags {
  Start {
    port: i32,
    host: String,
    node_capacity: i32,
  },
  Run {
    arweave_host: String,
    arweave_protocol: String,
    arweave_port: i32,
    contract_id: String,
  },
  Unknown(String),
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

  let flags = match pargs.subcommand()?.as_deref().unwrap_or("Unknown") {
    "start" => Flags::Start {
      port: pargs.opt_value_from_str("--port")?.unwrap_or(8755),
      host: pargs
        .opt_value_from_str("--host")?
        .unwrap_or(String::from("127.0.0.1")),
      node_capacity: parse_node_limit(&mut pargs).unwrap(),
    },
    "run" | _ => Flags::Run {
      arweave_protocol: pargs
        .opt_value_from_str("--arweave-protocol")?
        .unwrap_or(String::from("https")),
      arweave_host: pargs
        .opt_value_from_str("--arweave-host")?
        .unwrap_or(String::from("arweave.net")),
      arweave_port: pargs.opt_value_from_str("--arweave-port")?.unwrap_or(80),
      contract_id: pargs
        .opt_value_from_str("--contract-id")?
        .unwrap_or(String::from("t9T7DIOGxx4VWXoCEeYYarFYeERTpWIC1V3y-BPZgKE")),
    }, // any => Flags::Unknown(String::from(any)),
  };

  Ok(flags)
}
