pub fn current_node_tcp_ip(port: i32) -> String {
    let ip = local_ipaddress::get().unwrap_or(String::from("127.0.0.1"));
    String::from(format!("{}:{}", ip, port))
}
