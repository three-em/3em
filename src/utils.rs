use crate::node::Node;

pub fn parse_node_ip(node: &Node) -> String {
  parse_basic_ip(node.ip.to_owned(), node.port)
}

pub fn parse_basic_ip(ip: String, port: i32) -> String {
  format!("{}:{}", ip, port)
}

pub fn current_node_tcp_ip(port: i32) -> String {
  let ip = local_ipaddress::get().unwrap_or(String::from("127.0.0.1"));
  String::from(parse_basic_ip(ip, port))
}

pub fn usize_to_u8_array(num: usize) -> [u8; 4] {
  let mut bytes = [0; 4];
  bytes[0] = (num >> 24) as u8;
  bytes[1] = (num >> 16) as u8;
  bytes[2] = (num >> 8) as u8;
  bytes[3] = num as u8;
  bytes
}

pub fn u8_array_to_usize(bytes: [u8; 4]) -> usize {
  let mut num = 0;
  num |= (bytes[0] as usize) << 24;
  num |= (bytes[1] as usize) << 16;
  num |= (bytes[2] as usize) << 8;
  num |= bytes[3] as usize;
  num
}

#[cfg(test)]
mod tests {
  use crate::utils::{usize_to_u8_array, u8_array_to_usize};

  #[tokio::test]
  async fn test_usize_to_u8_array() {
    let message = "Hello".repeat(100000);
    let bytes = message.as_bytes();
    let len = bytes.len();
    assert_eq!(len, 500000 as usize);
    let to_u8_array = usize_to_u8_array(len);
    let expected: [u8; 4] = [0, 7, 161, 32];
    assert_eq!(to_u8_array, expected);
    assert_eq!(500000 as usize, u8_array_to_usize(to_u8_array));
    assert_eq!(usize_to_u8_array(500000), expected.to_owned());
  }

}
