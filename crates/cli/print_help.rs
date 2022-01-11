use indoc::indoc;

pub fn print_help(sub_command: Option<&str>) {
  let help = match sub_command.unwrap_or("none") {
    "dry-run" => indoc! {"
            three_em dry-run [options]

            Runs a local contract with local interactions provided in a configuration file.

            Options:
                --host   Gateway url to be used by SmartWeave APIs   (Default: arweave.net)   [string]
                --port   Gateway port to be used   (Default: 443)   [string]
                --protocol   Protocol to be used for gateway communication (Default: https)   [http|https]
                --pretty-print   Whether state result should be in JSON prettified form   (Default: false)   [boolean]
                --show-validity   Whether validity table should be included in output   (Default: false)   [boolean]
                --file   Path to configuration file to be used   (Required)   [string]
    "},
    "run" => indoc! {"
            three_em run [options]

            Runs a contract deployed to the Arweave network.

            Options:
                --contract-id  ID of contract to be evaluated   (Required)   [string]
                --host   Gateway url to be used by Executor & SmartWeave APIs   (Default: arweave.net)   [string]
                --port   Gateway port to be used   (Default: 443)   [string]
                --protocol   Protocol to be used for gateway communication (Default: https)   [http|https]
                --pretty-print   Whether state result should be in JSON prettified form   (Default: false)   [boolean]
                --show-validity   Whether validity table should be included in output   (Default: false)   [boolean]
                --no-print   Whether no output should be displayed   (Default: false)   [boolean]
                --benchmark   Whether execution time should be displayed   (Default: false)   [boolean]
                --no-cache   Whether cache system should be used for evaluation   (Default: true)   [boolean]
                --show-errors   Whether exceptions thrown during evaluation should be shown   (Default: false)   [boolean]
                --save   Path to file where output will be saved   [string]
                --height   Maximum height to be evaluated   [number]
    "},
    "none" | _ => indoc! {"
            three_em <command> [options]

            Commands:
                three_em run [options]   Evaluates the latest state of a deployed contract.
                three_em dry-run [options]   Evaluates the latest state of a local contract.
    "},
  };

  println!("{}", help);
}
