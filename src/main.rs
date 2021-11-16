mod cli_flags;

use crate::cli_flags::cli_flags::CliOperator;
use deno_core::error::AnyError;
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

    let cli_operator = CliOperator {};
    let flags = cli_operator.parse();

    let specifier = format!("{}:{}", flags.host, flags.port);
    println!("Serving {}", &specifier);

    let listener = TcpListener::bind(specifier).await?;

    loop {
        let (socket, _) = listener.accept().await?;

        tokio::task::spawn(async move {
            handle_node(socket).await;
        });
    }
}
