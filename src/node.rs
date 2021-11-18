use std::net::TcpStream;
use std::io::{Write, Read};

pub struct Node {
    ip: String,
    port: i32
}

pub async fn send_message(message: String, node: &Node) -> Result<Vec<u8>, &'static str> {
    match TcpStream::connect(format!("{}:{}", node.ip, node.port)) {
        Ok(mut stream) => {
            let future = tokio::task::spawn(async move {
                stream.write(message.as_bytes()).unwrap();
                let mut result: Vec<u8> = Vec::new();

                loop {
                    let mut buf = [0; 1024];
                    let n = stream.read(&mut buf[..]).unwrap();

                    if n == 0 {
                        break;
                    }

                    result.extend_from_slice(&buf);
                }

                result
            });

            let result = future.await;
            Ok(result.unwrap())
        }
        Err(_) => {
            Err("Could not send message")
        }
    }
}
