use crate::core_nodes::get_core_nodes;
use crate::messages::get_addr::get_addr;
use crate::node::{send_message, Node};
use deno_core::error::AnyError;
use deno_core::futures::pin_mut;
use deno_core::futures::stream::poll_fn;
use deno_core::futures::stream::unfold;
use deno_core::futures::stream::Stream;
use deno_core::futures::task::Poll;
use deno_core::futures::StreamExt;
use std::future::Future;
use std::pin::Pin;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::OwnedReadHalf;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

/// A stream of incoming data from a TCP socket.
///
/// The data is represented as follows:
/// |<--- 4 bytes ---> | <--- n bytes --->|   <--- 1 byte --->  |
/// |  length of data  |  actual data     | 0x69 - magic number |
///
/// If the magic number is not 0x69, the data is invalid.
fn handle_node(mut stream: TcpStream) -> Pin<Box<impl Stream<Item = Vec<u8>>>> {
  let stream = unfold(stream, |mut stream| async {
    let mut buf = [0; 1024];

    let mut len = [0; 4];
    let read = stream.read(&mut len).await.unwrap();

    let mut len_u32 = u32::from_le_bytes(len);
    let mut data = vec![0; len_u32 as usize];
    let read = stream.read(&mut data).await.unwrap();

    println!("read {}b of data", read);
    let mut magic = [0; 1];
    let read = stream.read(&mut magic).await.unwrap();

    assert_eq!(magic[0], 0x69);
    Some((data, (stream)))
  });

  Box::pin(stream)
}

async fn process(inbound: Vec<u8>) {
  // TODO
}

async fn discover(host: &str, port: i32) {
  let node = Node::new(host, port);
  send_message(String::from("Hello"), &node).await;
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
      for data in handle_node(socket).next().await {
        process(data).await;
      }
    });
  }
}
