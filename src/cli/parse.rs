use pico_args::Arguments;

#[derive(Debug)]
pub enum Flags {
    Start { port: i32, host: String },
    Unknown,
}

pub fn parse() -> Result<Flags, pico_args::Error> {
    let mut pargs = Arguments::from_env();

    let flags = match pargs.subcommand()?.as_deref() {
        Some("start") => Flags::Start {
            port: pargs.opt_value_from_str("--port")?.unwrap_or(8755),
            host: pargs
                .opt_value_from_str("--host")?
                .unwrap_or(String::from("127.0.0.1")),
        },
        _ => Flags::Unknown,
    };

    Ok(flags)
}
