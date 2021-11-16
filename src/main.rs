mod cli;
mod runtime;
mod start;

use cli::parse::Flags;
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
Languages supported: Javascript, Rust, C++, C, C#.
"#;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
  println!("{}", BANNER);
  println!("Version: {}", env!("CARGO_PKG_VERSION"));

  let flags = cli::parse::parse()?;

  match flags {
    Flags::Start { host, port } => {
      start::start(host, port).await?;
    }
    _ => {
      println!("{}", "Unknown flag.");
    }
  };

  Ok(())
}
