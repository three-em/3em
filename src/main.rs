mod vem_core;
mod cli_flags;

use std::{thread, env};
use std::net::TcpStream;
use std::io::{Read};
use vem_core::core::VemCore;
use crate::cli_flags::cli_flags::CliOperator;

fn handle_node(mut stream: TcpStream){
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
    let cli_operator = CliOperator {};
    let flags = cli_operator.parse();

    let core = VemCore {
        ip: flags.host.unwrap_or("127.0.0.1".to_owned()),
        port: flags.port.unwrap_or(8755),
    };

    let listener = core.begin();

    for stream in listener.incoming() {
        thread::spawn(|| {
            match stream {
                Ok(stream)=> {
                    handle_node(stream);
                }
                Err(_e)=> {
                    println!("A connection was received but failed to be handled.")
                }
            }
        });
    }
}
