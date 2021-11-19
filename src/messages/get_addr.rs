use crate::node::Node;

pub fn get_addr(my_node: &Node) -> String {
  format!(
    "host:{}\nversion:{}",
    my_node.to_string(),
    env!("CARGO_PKG_VERSION")
  )
}
