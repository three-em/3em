use pico_args::Arguments;

#[derive(Debug)]
pub enum Flags {
  Start {
    port: i32,
    host: String,
    node_capacity: i32,
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
    "start" | _ => Flags::Start {
      port: pargs.opt_value_from_str("--port")?.unwrap_or(8755),
      host: pargs
        .opt_value_from_str("--host")?
        .unwrap_or(String::from("127.0.0.1")),
      node_capacity: parse_node_limit(&mut pargs).unwrap(),
    },
    // any => Flags::Unknown(String::from(any)),
  };

  Ok(flags)
}
