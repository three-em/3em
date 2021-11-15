use std::collections::HashMap;
use std::env;
use std::str::FromStr;

pub struct CliContext {
    command: Option<String>,
    flags: HashMap<String, Vec<String>>,
}

pub struct CliOperator;
impl CliOperator {
    pub fn new() -> CliOperator {
        CliOperator {}
    }

    pub fn parse(&self, args: Vec<String>) -> Option<HashMap<String, Vec<(String, String)>>> {
        let args: Vec<String> = args.into_iter().skip(1).collect();
        let command = args.get(0).and_then(|val| Some(val.to_owned()));

        let mut valid_commands: HashMap<String, Vec<&str>> = HashMap::new();
        self.initialize_start_cmd(&mut valid_commands);

        match command {
            Some(cmd) => {
                let mut result_vec: Vec<(String, String)> = Vec::new();
                let mut result_map: HashMap<String, Vec<(String, String)>> = HashMap::new();
                let flags: &Vec<String> = &args.into_iter().skip(1).collect();
                let known_flags: Vec<&str> = valid_commands.get(&cmd).unwrap().clone();

                for known_flg in known_flags {
                    let flag = known_flg.to_owned();
                    result_vec.push(self.get_arg_val(&flags, flag));
                }

                result_map.insert(cmd, result_vec);

                Some(result_map)
            }
            None => None,
        }
    }

    fn initialize_start_cmd(&self, commands: &mut HashMap<String, Vec<&str>>) {
        commands.insert("start".to_owned(), vec!["--host", "--port"]);
    }

    fn get_arg_val(&self, args: &Vec<String>, arg: String) -> (String, String) {
        let arg_key_index = args.iter().position(|r| r.to_owned() == arg).unwrap_or(0);
        let arg_key_value = arg_key_index + 1;
        let arg_value = String::from(args.get(arg_key_value).unwrap_or(&("".to_string())));
        return (arg, arg_value);
    }
}

#[cfg(test)]
mod cli_flags_tests {
    use crate::cli_flags::cli_flags::CliOperator;

    #[test]
    fn parse_flags() {
        let operator = CliOperator::new();
        let mut flags: Vec<String> = Vec::new();
        flags.push("vec".to_owned());
        flags.push("start".to_owned());
        flags.push("--host".to_owned());
        flags.push("127.0.0.1".to_owned());

        let parse_flags = operator.parse(flags).unwrap();
        assert_eq!(parse_flags.contains_key("start"), true);

        let start_flag = parse_flags.get("start").unwrap();
        let (host_arg, host_val) = start_flag.get(0).unwrap();
        assert_eq!(host_arg, "--host");
        assert_eq!(host_val, "127.0.0.1");
    }
}
