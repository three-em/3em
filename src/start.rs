use crate::core_nodes::get_core_nodes;
use crate::messages::get_addr::get_addr;
use crate::node::{send_message, Node};
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

async fn discover(host: &str, port: i32) {
  let node = Node::new(host, port);
  send_message(String::from("Hello"), &node);
}

async fn send_discovery(nodes: &Vec<Node>) {
  for node in nodes {
    let message = get_addr(node);
    let result = send_message(message, node).await.unwrap();
    // TODO: Verify result is a pong message containing the same output from get_addr
    // TODO: If the response matches get_addr (host and version information), add the node to a list of nodes that answered the call.
  }
}

pub async fn start(
  host: String,
  port: i32,
  node_capacitiy: i32,
) -> Result<(), AnyError> {
  let specifier = format!("{}:{}", host, port);
  let this_node = Node::new(&host, port);

  println!("Serving {}", &specifier);

  let core_nodes: Vec<Node> = get_core_nodes()
    .into_iter()
    .filter(|node| node.is_not(&this_node))
    .collect();

  send_discovery(&core_nodes).await;

  let listener = TcpListener::bind(specifier).await?;

  loop {
    let (socket, _) = listener.accept().await?;

    tokio::task::spawn(async move {
      handle_node(socket).await;
    });
  }
}
