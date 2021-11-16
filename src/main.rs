mod cli_commands;
mod cli_flags;
mod vem_core;

use crate::cli_commands::start_cmd::StartCmd;
use crate::cli_flags::cli_flags::CliOperator;
use std::io::Read;
use std::net::TcpStream;
use std::{env, thread};
use vem_core::core::VemCore;

pub fn main() {
    let mut cli_operator = CliOperator::new();
    let mut arguments: Vec<String> = env::args().collect();
    if arguments.len() == 1 {
        arguments = vec!["vem", "start", "--host", "127.0.0.1", "--port", "8755"]
            .iter()
            .map(|x| String::from(x.to_owned()))
            .collect();
    }

    cli_operator.on_trait(StartCmd);

    cli_operator.begin(arguments);
}
