mod cli_flags;
mod vem_core;

use crate::cli_flags::cli_flags::CliOperator;
use std::io::Read;
use std::net::TcpStream;
use std::{env, thread};
use vem_core::core::VemCore;

fn handle_node(mut stream: TcpStream) {
    loop {
        let mut buf = [0; 1024];
        let n = stream.read(&mut buf[..]).unwrap();
        eprintln!("read {}b of data", n);
        if n == 0 {
            eprintln!("no more data!");
            break;
        } else {
            println!("{}", std::str::from_utf8(&buf[..n]).unwrap());
        }
    }
}

pub fn main() {
    let mut cli_operator = CliOperator::new();
    let mut arguments: Vec<String> = env::args().collect();
    if arguments.len() == 1 {
        arguments = vec!["vem", "start", "--host", "127.0.0.1", "--port", "8755"]
            .iter()
            .map(|x| String::from(x.to_owned()))
            .collect();
    }

    cli_operator.on("start", |flags| {
        let core = VemCore {
            ip: flags.get("--host").unwrap().to_owned(),
            port: flags.get("--port").unwrap().parse::<i32>().unwrap(),
        };

        let listener = core.begin();

        for stream in listener.incoming() {
            thread::spawn(|| match stream {
                Ok(stream) => {
                    handle_node(stream);
                }
                Err(_e) => {
                    println!("A connection was received but failed to be handled.")
                }
            });
        }
    });

    cli_operator.begin(arguments);
}
