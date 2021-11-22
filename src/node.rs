use crate::utils::{parse_node_ip, usize_to_u8_array};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::TcpStream;

#[derive(Serialize, Deserialize)]
pub struct Node {
  pub ip: String,
  pub port: i32,
}

impl Node {
  pub fn new(host: &str, port: i32) -> Node {
    Node {
      ip: String::from(host),
      port,
    }
  }

  pub fn is_not(&self, node: &Node) -> bool {
    let current_node = parse_node_ip(self);
    let diff_node = parse_node_ip(node);
    current_node != diff_node
  }

  pub fn to_string(&self) -> String {
    parse_node_ip(self)
  }
}

// TODO: Implement length approach
pub async fn send_message(
  message: String,
  node: &Node,
) -> Result<Vec<u8>, &str> {
  let result = match TcpStream::connect(node.to_string()) {
    Ok(mut stream) => {
      let future = tokio::task::spawn(async move {
        let message_as_bytes = message.as_bytes();
        let message_len = message_as_bytes.len();
        let message_length = &usize_to_u8_array(message_len.to_owned());
        let magic_number = 0x69 as u8;

        let mut final_message: Vec<u8> = Vec::new();
        final_message.extend_from_slice(message_length);
        final_message.extend_from_slice(message_as_bytes);
        final_message.extend_from_slice(&[magic_number]);


        let as_bytes = &final_message[..];
        stream.write(as_bytes).unwrap();
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
    Err(_) => Err("Could not send message"),
  };

  result
}

#[cfg(test)]
mod tests {
  use crate::node::Node;

  #[tokio::test]
  async fn test_is_not() {
    let node1 = Node::new("127.0.0.1", 9999);
    let node2 = Node::new("127.0.0.1", 9898);
    assert!(node1.is_not(&node2));

    let node1 = Node::new("127.0.0.1", 9999);
    let node2 = Node::new("127.0.0.1", 9999);
    assert!(!(node1.is_not(&node2)));
  }

  #[tokio::test]
  async fn test_to_string() {
    let node1 = Node::new("127.0.0.1", 9999);
    assert_eq!(node1.to_string(), "127.0.0.1:9999");
  }
}
