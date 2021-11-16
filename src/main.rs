mod cli_commands;
mod cli_flags;

use crate::cli_flags::cli_flags::CliOperator;
use deno_core::error::AnyError;

use crate::cli_commands::start_cmd::StartCmd;
use std::io::Read;
use std::net::TcpStream;
use std::{env, thread};

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

    let mut cli_operator = CliOperator::new();
    let mut arguments: Vec<String> = env::args().collect();
    if arguments.len() == 1 {
        arguments = vec!["vem", "start", "--host", "127.0.0.1", "--port", "8755"]
            .iter()
            .map(|x| String::from(x.to_owned()))
            .collect();
    }

    cli_operator.on_trait(StartCmd);

    cli_operator.begin(arguments).await;

    Ok(())
}
