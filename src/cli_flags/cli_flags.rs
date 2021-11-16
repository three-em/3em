use std::collections::HashMap;
use std::env;
use std::str::FromStr;

pub trait CliHandler {
    fn get_command(&self) -> &str;
    fn execute(&self, flags: HashMap<String, String>) -> ();
}

pub struct CliContext {
    command: String,
    flags: HashMap<String, String>,
}

pub struct CliOperator<'a> {
    executors: HashMap<String, Box<dyn Fn(HashMap<String, String>) + 'a>>,
    trait_executors: HashMap<String, Box<dyn CliHandler + 'a>>,
}

impl<'a> CliOperator<'a> {
    pub fn new() -> CliOperator<'a> {
        CliOperator {
            executors: HashMap::new(),
            trait_executors: HashMap::new(),
        }
    }

    pub fn parse(&self, args: Vec<String>) -> Option<CliContext> {
        let args: Vec<String> = args.into_iter().skip(1).collect();
        let command = args.get(0).and_then(|val| Some(val.to_owned()));

        let mut valid_commands: HashMap<String, Vec<&str>> = HashMap::new();
        self.initialize_start_cmd(&mut valid_commands);

        match command {
            Some(cmd) => {
                let mut result_flags: HashMap<String, String> = HashMap::new();
                let mut context: CliContext = CliContext {
                    command: String::from(cmd.clone()),
                    flags: HashMap::new(),
                };
                let flags: &Vec<String> = &args.into_iter().skip(1).collect();
                let known_flags: Vec<&str> = valid_commands.get(&cmd).unwrap().clone();

                for known_flg in known_flags {
                    let flag = known_flg.to_owned();
                    let (arg, arg_val) = self.get_arg_val(&flags, flag);
                    result_flags.insert(arg, arg_val);
                }

                context.flags = result_flags;

                Some(context)
            }
            None => None,
        }
    }

    pub fn on<F: Fn(HashMap<String, String>) + 'a>(&mut self, command: &str, executor: F) {
        self.executors
            .insert(String::from(command), Box::new(executor));
    }

    pub fn on_trait<Handler: CliHandler + 'a>(&mut self, handler: Handler) {
        self.trait_executors
            .insert(String::from(handler.get_command()), Box::new(handler));
    }

    pub fn begin(&self, args: Vec<String>) {
        // TODO: Don't panic if command does not exist
        let parse = self.parse(args).unwrap();
        if let Some(executor) = self.executors.get(&parse.command) {
            executor(parse.flags);
        } else if let Some(executor) = self.trait_executors.get(&parse.command) {
            executor.execute(parse.flags);
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
    use crate::cli_flags::cli_flags::{CliHandler, CliOperator};
    use std::collections::HashMap;

    #[test]
    fn parse_flags() {
        let operator = CliOperator::new();
        let mut flags: Vec<String> = Vec::new();
        flags.push("3em".to_owned());
        flags.push("start".to_owned());
        flags.push("--host".to_owned());
        flags.push("127.0.0.1".to_owned());

        let parse_flags = operator.parse(flags).unwrap();
        assert_eq!(parse_flags.command, "start");
        let host_val = parse_flags.flags.get("--host").unwrap();
        assert_eq!(host_val, "127.0.0.1");
    }

    #[test]
    fn execute_hello_world() {
        let mut operator = CliOperator::new();
        let mut flags: Vec<String> = Vec::new();
        flags.push("3em".to_owned());
        flags.push("start".to_owned());
        flags.push("--host".to_owned());
        flags.push("127.0.0.1".to_owned());

        operator.on("start", |data| {
            let result = format!("Hello world from {}", data.get("--host").unwrap());
            assert_eq!(result, "Hello world from 127.0.0.1");
        });

        operator.begin(flags);
    }

    pub struct StartCommand;
    impl CliHandler for StartCommand {
        fn get_command(&self) -> &str {
            return "start";
        }

        fn execute(&self, flags: HashMap<String, String>) -> () {
            let result = format!("Hello world from {}", flags.get("--host").unwrap());
            assert_eq!(result, "Hello world from 127.0.0.1");
        }
    }

    #[test]
    fn execute_hello_world_trait() {
        let mut operator = CliOperator::new();
        let mut flags: Vec<String> = Vec::new();
        flags.push("3em".to_owned());
        flags.push("start".to_owned());
        flags.push("--host".to_owned());
        flags.push("127.0.0.1".to_owned());

        operator.on_trait(StartCommand);

        operator.begin(flags);
    }
}
