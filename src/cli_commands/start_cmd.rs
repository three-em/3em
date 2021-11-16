use crate::cli_flags::cli_flags::CliHandler;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

async fn handle_node(mut stream: TcpStream) {
    loop {
        let mut buf = [0; 1024];

        let n = stream.read(&mut buf[..]).await.unwrap();
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

#[async_trait]
impl CliHandler for StartCmd {
    fn get_command(&self) -> &str {
        "start"
    }

    async fn execute(&self, flags: HashMap<String, String>) -> () {
        let specifier = format!(
            "{}:{}",
            flags.get("--host").unwrap().to_owned(),
            flags.get("--port").unwrap().parse::<i32>().unwrap()
        );
        println!("Serving {}", &specifier);

        let listener = TcpListener::bind(specifier).await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();

            tokio::task::spawn(async move {
                handle_node(socket).await;
            });
        }
    }
}
