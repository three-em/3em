use crate::cli_flags::cli_flags::CliHandler;
use crate::vem_core::core::VemCore;
use std::collections::HashMap;
use std::io::Read;
use std::net::TcpStream;
use std::thread;

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

pub struct StartCmd;
impl CliHandler for StartCmd {
    fn get_command(&self) -> &str {
        return "start";
    }

    fn execute(&self, flags: HashMap<String, String>) -> () {
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
    }
}
