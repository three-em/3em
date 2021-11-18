pub static CORE_NODES: &'static str = include_str!("metadata/core_nodes.txt");

pub fn get_core_nodes() -> Vec<String> {
    let nodes: Vec<String> = CORE_NODES.to_owned().split("\n").map(|node| String::from(node)).collect();
    let nodes = nodes.iter().filter(|&p| !(p.eq(""))).cloned().collect();
    nodes
}
