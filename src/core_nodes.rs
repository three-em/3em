use crate::node::Node;

pub static CORE_NODES: &'static str = include_str!("metadata/core_nodes.txt");

pub fn get_core_nodes() -> Vec<Node> {
  let nodes: Vec<String> = CORE_NODES
    .to_owned()
    .split("\n")
    .map(|node| String::from(node))
    .collect();
  let nodes: Vec<String> =
    nodes.iter().filter(|&p| !(p.eq(""))).cloned().collect();
  let result = nodes
    .iter()
    .map(|content| {
      let data: Vec<String> =
        content.split(":").map(|node| String::from(node)).collect();
      let ip = data.get(0).unwrap();
      let port = data.get(1).unwrap().parse::<i32>().unwrap();
      Node::new(ip, port)
    })
    .collect();
  result
}
