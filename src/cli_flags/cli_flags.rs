use std::env;
use std::str::FromStr;

pub struct CliFlags {
    pub host: String,
    pub port: i32,
}

pub struct CliOperator;
impl CliOperator {
    pub fn parse<'a>(&self) -> CliFlags {
        let args: Vec<String> = env::args().collect();
        let known_args: Vec<&str> = vec!["--host", "--port"];

        let mut result = CliFlags {
            host: "127.0.0.1".to_string(),
            port: 8755,
        };

        for x in known_args {
            let value = self.get_arg_val(&args, x);
            if let Some(value) = value {
                match x {
                    "--host" => {
                        result.host = value;
                    }
                    "--port" => {
                        result.port = i32::from_str(&value).expect("Invalid port.");
                    }
                    _ => {}
                }
            }
        }

        result
    }

    fn get_arg_val<'a>(&self, args: &'a [String], arg: &str) -> Option<String> {
        let arg_key_index = args.iter().position(|r| r == arg).unwrap_or(0);
        let arg_key_value = arg_key_index + 1;
        args.get(arg_key_value).cloned()
    }
}
