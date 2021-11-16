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

pub async fn start(host: String, port: i32) -> Result<(), AnyError> {
  let specifier = format!("{}:{}", host, port);

  println!("Serving {}", &specifier);

  let listener = TcpListener::bind(specifier).await?;

  loop {
    let (socket, _) = listener.accept().await?;

    tokio::task::spawn(async move {
      handle_node(socket).await;
    });
  }
}
