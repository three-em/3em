use std::net::TcpStream;
use std::io::{Write, Read};
use crate::utils::parse_node_ip;

pub struct Node {
    pub ip: String,
    pub port: i32
}

impl Node {
    pub fn new(host: &str, port: i32) -> Node {
        Node {
            ip: String::from(host),
            port
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

pub async fn send_message(message: String, node: &Node) -> Result<Vec<u8>, &str> {
    let result = match TcpStream::connect(format!("{}:{}", node.ip, node.port)) {
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
