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
