mod cli;
mod runtime;
mod snapshot_js;
mod start;

use cli::parse::Flags;
use deno_core::error::AnyError;

use colored::Colorize;
use std::env;

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
    Flags::Start { host, port } => {
      start::start(host, port).await?;
    }
    Flags::Unknown(cmd) => {
      print_cmd_error(&cmd);
    }
  };

  Ok(())
}

fn print_cmd_error(cmd: &String) {
  println!("{}: Found argument '{}' which wasn't expected, or isn't valid in this context.", "error".red(), cmd);
  println!("{}", USAGE);
}
