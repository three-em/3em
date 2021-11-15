use std::env;
use std::str::FromStr;

pub struct CliFlags {
    pub host: Option<String>,
    pub port: Option<i32>
}

pub struct CliOperator;
impl CliOperator {
    pub fn parse(&self) -> CliFlags {
        let args: Vec<String> = env::args().collect();
        let known_args: Vec<&str> = vec![
            "--host",
            "--port"
        ];

        let mut result = CliFlags {
          host: None,
          port: None
        };

        for x in known_args {
            let (_, value) = self.get_arg_val(&args, x.to_owned());
            if value != "" && x == "--host" {
                result.host = Some(value.to_owned());
            }
            if value != "" && x == "--port" {
                result.port = Some(i32::from_str(&value).unwrap_or(8755));
            }
        }

        result
    }

    fn get_arg_val(&self, args: &Vec<String>, arg: String) -> (String, String) {
        let arg_key_index = args.iter().position(|r| r.to_owned() == arg).unwrap_or(0);
        let arg_key_value = arg_key_index + 1;
        let arg_value = String::from(args.get(arg_key_value).unwrap_or(&("".to_string())));
        return (arg, arg_value)
    }

}
