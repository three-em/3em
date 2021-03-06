use crate::node::Node;
use sha2::Digest;

pub fn parse_node_ip(node: &Node) -> String {
  parse_basic_ip(node.ip.to_string(), node.port)
}

pub fn parse_basic_ip(ip: String, port: i32) -> String {
  format!("{}:{}", ip, port)
}

pub fn usize_to_u8_array(num: u32) -> [u8; 4] {
  u32::to_le_bytes(num)
}

pub fn u8_array_to_usize(bytes: [u8; 4]) -> usize {
  u32::from_le_bytes(bytes) as usize
}

pub fn hasher(data: &[u8]) -> Vec<u8> {
  let mut hasher = sha2::Sha256::new();
  hasher.update(data);
  hasher.finalize()[..].to_vec()
}

#[cfg(test)]
mod tests {
  use crate::utils::{u8_array_to_usize, usize_to_u8_array};

  #[tokio::test]
  async fn test_usize_to_u8_array() {
    let message = "Hello".repeat(100000);
    let bytes = message.as_bytes();
    let len = bytes.len();
    assert_eq!(len, 500000_usize);
    let to_u8_array = usize_to_u8_array(len as u32);
    let expected: [u8; 4] = [32, 161, 7, 0];
    assert_eq!(to_u8_array, expected);
    assert_eq!(500000_usize, u8_array_to_usize(to_u8_array));
    assert_eq!(usize_to_u8_array(500000), expected.to_owned());
  }
}
