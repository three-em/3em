use std::net::TcpListener;

pub struct VemCore {
    pub ip: String,
    pub port: i32,
}

impl VemCore {
    pub fn begin(&self) -> TcpListener {
        let listener = TcpListener::bind(format!("{}:{}", &self.ip, &self.port)).unwrap();
        self.start_up();
        listener
    }

    fn start_up(&self) {
        println!("██████╗     ███████╗    ███╗   ███╗");
        println!("╚════██╗    ██╔════╝    ████╗ ████║");
        println!(" █████╔╝    █████╗      ██╔████╔██║");
        println!(" ╚═══██╗    ██╔══╝      ██║╚██╔╝██║");
        println!("██████╔╝    ███████╗    ██║ ╚═╝ ██║");
        println!("╚═════╝     ╚══════╝    ╚═╝     ╚═╝");
        println!();
        println!("The Web3 Execution Machine");
        println!("Languages supported: Javascript, Rust, C++, C, C#.");
        println!("{}", format!("Version: {}", env!("CARGO_PKG_VERSION")));
        println!();
        println!();
        println!("{}", format!("Serving {}:{}", &self.ip, &self.port));
    }
}
