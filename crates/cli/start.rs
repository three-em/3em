use crate::core_nodes::get_core_nodes;
use crate::messages::get_addr::get_addr;
use crate::node::{send_message, Node};
use crate::utils::u8_array_to_usize;
use deno_core::error::AnyError;
use deno_core::futures::stream::unfold;
use deno_core::futures::stream::Stream;
use deno_core::futures::StreamExt;
use std::pin::Pin;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;

/// A stream of incoming data from a TCP socket.
///
/// The data is represented as follows:
/// |<--- 4 bytes ---> | <--- n bytes --->|   <--- 1 byte --->  |
/// |  length of data  |  actual data     | 0x69 - magic number |
///
/// If the magic number is not 0x69, the data is invalid.
fn handle_node(stream: TcpStream) -> Pin<Box<impl Stream<Item = Vec<u8>>>> {
  let stream = unfold(stream, |mut stream| async {
    let mut len = [0; 4];
    stream.read(&mut len).await.unwrap();
    let mut message_len = u8_array_to_usize(len);

    let mut inbound_data: Vec<u8> = vec![];

    loop {
      let mut buf = vec![0u8; message_len]; // Allocate strictly what the header indicated, then allocate the left overs.
      let n = stream.read(&mut buf).await.unwrap();
      message_len -= n;

      inbound_data.append(&mut buf);

      if n == 0 || message_len == 0 {
        break;
      }
    }

    let mut magic = [0; 1];
    stream.read(&mut magic).await.unwrap();
    assert_eq!(magic[0], 0x69);
    Some((inbound_data, (stream)))
  });

  Box::pin(stream)
}

async fn process(_inbound: Vec<u8>) {
  // TODO
}

#[allow(dead_code)]
async fn discover(host: &str, port: i32) {
  let node = Node::new(host, port);
  send_message(String::from("Hello"), &node).await.unwrap();
}

async fn send_discovery(nodes: &[Node]) {
  for node in nodes {
    let message = get_addr(node);
    let _result = send_message(message, node).await.unwrap();
    // TODO: Verify result is a pong message containing the same output from get_addr
    // TODO: If the response matches get_addr (host and version information), add the node to a list of nodes that answered the call.
  }
}

pub async fn start(
  host: String,
  port: i32,
  _node_capacitiy: i32,
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
