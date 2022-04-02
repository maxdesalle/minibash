use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{stdout, Write};
use std::path::Path;
use std::process::{exit, Child, Command, Stdio};

static mut RUNNING_PROCESS_PID: i32 = 0;

#[derive(PartialEq, Debug, Clone)]
pub enum Separator {
    Ampersand, // &&
    Pipe,      // |
    Empty,
    SemiColon,              // ;
    WriteRedirection,       // >
    WriteAppendRedirection, // >>
}

pub struct InputOutput {
    pub file: Option<File>,
    pub stdout: Stdio,
    pub output: Option<Child>,
}

#[derive(Clone)]
pub struct CommandObject {
    pub text: String,
    pub separator: Separator,
    pub status_code: i32,
}

fn cd_update_env(env: &mut HashMap<String, String>) {
    let oldpwd = match env.get("PWD") {
        Some(oldpwd) => oldpwd.to_string(),
        None => return,
    };

    let pwd = env::current_dir().expect("Cannot find current directory");
    let pwd = match pwd.into_os_string().into_string() {
        Ok(pwd) => pwd,
        Err(_) => return,
    };

    update_env_variable(env, ("OLDPWD".to_string(), oldpwd));
    update_env_variable(env, ("PWD".to_string(), pwd));
}

fn cd_hyphen(env: &mut HashMap<String, String>, arg: &mut String) {
    *arg = match env.get("OLDPWD") {
        Some(arg) => {
            println!("{}", arg);
            arg.to_string()
        }
        None => {
            eprintln!("$OLDPWD environment variable not set");
            return;
        }
    }
}

fn cd_tilde(env: &mut HashMap<String, String>, arg: &mut String) {
    let home = match env.get("HOME") {
        Some(home) => home.to_string(),
        None => {
            eprintln!("$HOME environment variable not set");
            return;
        }
    };
    *arg = home.as_str().to_owned() + &arg[1..];
    *arg = arg.to_string();
}

fn cd(env: &mut HashMap<String, String>, arg: &mut String) {
    match arg.chars().next().unwrap() {
        '~' => cd_tilde(env, arg),
        '-' => cd_hyphen(env, arg),
        _ => {}
    }
    if let Err(e) = env::set_current_dir(&Path::new(arg)) {
        eprintln!("{}", e);
    } else {
        cd_update_env(env);
    }
}

fn export_no_args(env: &mut HashMap<String, String>, input_output: InputOutput) {
    let mut sorted: Vec<_> = env.iter().collect();

    let mut output = match input_output.file {
        Some(output) => Box::new(output) as Box<dyn Write>,
        None => Box::new(stdout()) as Box<dyn Write>,
    };

    sorted.sort_by_key(|a| a.0);
    for (key, value) in sorted {
        writeln!(output, "declare -x {}=\"{}\"", key, value)
            .unwrap_or_else(|err| println!("{:?}", err));
    }
}

fn update_env_variable(env: &mut HashMap<String, String>, key_value: (String, String)) {
    *env.get_mut(&key_value.0).unwrap() = key_value.1;
}

fn export_with_args(env: &mut HashMap<String, String>, args: &mut Vec<String>) {
    for arg in args {
        let mut key_value = arg.split('=');

        let key = key_value.next().unwrap();
        let value = key_value.next().unwrap();

        env.insert(key.to_string(), value.to_string());
    }
}

fn unset(env: &mut HashMap<String, String>, args: &mut Vec<String>) {
    for arg in args {
        env.remove(arg);
    }
}

fn print_env(env: &mut HashMap<String, String>, input_output: InputOutput) {
    let mut output = match input_output.file {
        Some(output) => Box::new(output) as Box<dyn Write>,
        None => Box::new(stdout()) as Box<dyn Write>,
    };

    for (key, value) in env {
        writeln!(output, "{}={}", key, value).unwrap_or_else(|err| println!("{:?}", err));
    }
}

pub fn save_env() -> HashMap<String, String> {
    let mut env = HashMap::new();

    for (key, value) in env::vars_os() {
        if let (Ok(k), Ok(v)) = (key.into_string(), value.into_string()) {
            env.insert(k, v);
        }
    }

    return env;
}

pub fn exit_handler(args: &mut Vec<String>) {
    println!("exit");
    let status_code = if args.len() == 0 {
        0
    } else {
        match args[0].parse() {
            Ok(status_code) => status_code,
            Err(_) => {
                eprintln!("minibash: exit: {}: numeric argument required", args[0]);
                255
            }
        }
    };
    exit(status_code);
}

fn print_var(env: &mut HashMap<String, String>, variable: &str, input_output: InputOutput) {
    let mut output = match input_output.file {
        Some(output) => Box::new(output) as Box<dyn Write>,
        None => Box::new(stdout()) as Box<dyn Write>,
    };

    match env.get(variable) {
        Some(var) => {
            writeln!(output, "{}", var.to_string()).unwrap_or_else(|err| println!("{:?}", err))
        }
        None => writeln!(output, "${} environment variable not set", variable)
            .unwrap_or_else(|err| println!("{:?}", err)),
    };
}

pub fn cd_redirector(env: &mut HashMap<String, String>, args: &mut Vec<String>) {
    if args.len() == 0 {
        let mut path = match env.get("HOME") {
            Some(path) => path.to_string(),
            None => {
                eprintln!("$HOME environment variable not set");
                return;
            }
        };
        cd(env, &mut path);
    } else {
        cd(env, &mut args[0]);
    }
}

pub fn unset_redirector(env: &mut HashMap<String, String>, args: &mut Vec<String>) {
    if args.len() == 0 {
        return;
    } else {
        unset(env, args);
    }
}

pub fn export_redirector(
    env: &mut HashMap<String, String>,
    args: &mut Vec<String>,
    input_output: InputOutput,
) {
    if args.len() == 0 {
        export_no_args(env, input_output);
    } else {
        export_with_args(env, args);
    }
}

fn echo_option_n(args: &mut Vec<String>, mut output: Box<dyn Write>) {
    let mut i = 1;

    while i < args.len() {
        write!(output, "{}", args[i]).unwrap_or_else(|err| println!("{:?}", err));
        if i != args.len() - 1 {
            write!(output, " ").unwrap_or_else(|err| println!("{:?}", err));
        }
        i += 1;
    }
}

pub fn echo_handler(args: &mut Vec<String>, input_output: InputOutput) {
    let mut i = 0;

    let mut output = match input_output.file {
        Some(output) => Box::new(output) as Box<dyn Write>,
        None => Box::new(stdout()) as Box<dyn Write>,
    };

    while i < args.len() {
        if i == 0 && args[i].as_str() == "-n" {
            echo_option_n(args, output);
            return;
        }
        write!(output, "{}", args[i]).unwrap_or_else(|err| println!("{:?}", err));
        if i == args.len() - 1 {
            writeln!(output, "").unwrap_or_else(|err| println!("{:?}", err));
        } else {
            write!(output, " ").unwrap_or_else(|err| println!("{:?}", err));
        }
        i += 1;
    }
}

fn execute_command(
    command: &mut CommandObject,
    executable: String,
    args: &mut Vec<String>,
    input_output: InputOutput,
) -> Option<Child> {
    let child = Command::new(executable)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(input_output.stdout)
        .spawn();

    match child {
        Ok(mut child) => {
            unsafe {
                RUNNING_PROCESS_PID = child.id() as i32;
            }
            match child.wait() {
                Ok(status) => unsafe {
                    RUNNING_PROCESS_PID = 0;
                    match status.code() {
                        Some(status) => {
                            command.status_code = status;
                        }
                        None => {
                            command.status_code = 0;
                        }
                    }
                    return Some(child);
                },
                Err(e) => eprintln!("{}", e),
            };
        }
        Err(e) => eprintln!("{}", e),
    }
    return None;
}

pub fn command_matcher(
    env: &mut HashMap<String, String>,
    args: &mut Vec<String>,
    command: &mut CommandObject,
    input_output: InputOutput,
) -> Option<Child> {
    let executable = args.remove(0);

    match executable.as_str() {
        "cd" => cd_redirector(env, args),
        "clear" => print!("\x1B[2J\x1B[1;1H"),
        "echo" => echo_handler(args, input_output),
        "env" => print_env(env, input_output),
        "exit" => exit_handler(args),
        "export" => export_redirector(env, args, input_output),
        "pwd" => print_var(env, "PWD", input_output),
        "unset" => unset_redirector(env, args),
        _ => {
            return execute_command(command, executable, args, input_output);
        }
    }
    return None;
}

pub fn splitter(input: &String) -> Vec<String> {
    let mut i = 0;
    let mut vec: Vec<String> = Vec::new();

    while i + 1 < input.len() {
        let mut arg = String::new();

        if input.chars().nth(i).unwrap() == '"' {
            i += 1;
            while input.chars().nth(i).unwrap() != '"' && i + 1 < input.len() {
                arg.push(input.chars().nth(i).unwrap());
                i += 1;
            }
        } else if input.chars().nth(i).unwrap() == '\'' {
            i += 1;
            while input.chars().nth(i).unwrap() != '\'' && i + 1 < input.len() {
                arg.push(input.chars().nth(i).unwrap());
                i += 1;
            }
        } else {
            while i < input.len() && input.chars().nth(i).unwrap() != ' ' {
                if input.chars().nth(i).unwrap() == '"' {
                    if i + 1 < input.len() {
                        i += 1;
                    } else {
                        break;
                    }
                    while i < input.len() && input.chars().nth(i).unwrap() != '"' {
                        arg.push(input.chars().nth(i).unwrap());
                        i += 1;
                    }
                } else {
                    arg.push(input.chars().nth(i).unwrap());
                    i += 1;
                }
            }
        }
        if !arg.is_empty() {
            vec.push(arg);
        }
        i += 1;
    }
    return vec;
}

fn invalid_char_check(c: char) -> bool {
    return c != ' ' && c != '\'' && c != '"';
}

pub fn dollar_expander(env: &mut HashMap<String, String>, input: String) -> String {
    let mut i = 0;
    let mut between_quotes = false;
    let mut input = input.clone();

    while i < input.len() {
        if input.chars().nth(i).unwrap() == '\'' && between_quotes == false {
            i += 1;
            while input.chars().nth(i).unwrap() != '\'' && i + 1 < input.len() {
                i += 1;
            }
        } else if input.chars().nth(i).unwrap() == '"' {
            if between_quotes == true {
                between_quotes = false;
            } else {
                between_quotes = true;
            }
        } else if input.chars().nth(i).unwrap() == '$' {
            let mut save = i + 1;
            while invalid_char_check(input.chars().nth(i).unwrap()) && i + 1 < input.len() {
                i += 1;
            }
            if input.chars().last().unwrap() != '"' {
                i += 1;
            }

            let var = env.get(&input[save..i]);
            match var {
                Some(var) => {
                    save -= 1;
                    input.replace_range(save..i, var);
                }
                None => {}
            }
        }
        i += 1;
    }
    return input;
}

#[allow(unused_variables)]
pub unsafe extern "C" fn handle_sigint(sig: libc::c_int) {
    if RUNNING_PROCESS_PID == 0 {
        println!();
        print!("minibash-3.2$ ");
        if let Err(error) = stdout().flush() {
            eprintln!("{}", error);
        }
    } else {
        libc::kill(RUNNING_PROCESS_PID, libc::SIGCONT);
        println!();
    }
}

#[allow(unused_variables)]
pub unsafe extern "C" fn handle_sigusr1(sig: libc::c_int) {
    println!("exit");
    exit(0);
}

#[allow(unused_variables)]
pub unsafe extern "C" fn handle_sigquit(sig: libc::c_int) {
    if RUNNING_PROCESS_PID == 0 {
        print!("{}", "\r\r");
        print!("minibash-3.2$ ");
        if let Err(error) = stdout().flush() {
            eprintln!("{}", error);
        }
    } else {
        libc::kill(RUNNING_PROCESS_PID, libc::SIGCONT);
        println!("Quit: 3");
    }
}

pub fn update_shlvl(env: &mut HashMap<String, String>) {
    let string_var = env.get("SHLVL");
    match string_var {
        Some(string_var) => {
            let int_var = string_var.parse::<i32>().unwrap() + 1;
            update_env_variable(env, ("SHLVL".to_string(), int_var.to_string()));
        }
        None => {
            env.insert("SHLVL".to_string(), "1".to_string());
        }
    }
}

pub fn arg_split(input: &mut String) -> Vec<CommandObject> {
    let mut i = 0;
    let mut j = 0;
    let mut commands: Vec<CommandObject> = Vec::new();

    while i < input.len() {
        if input.chars().nth(i).unwrap() == '\'' {
            i += 1;
            while i + 1 < input.len() && input.chars().nth(i).unwrap() != '\'' {
                i += 1;
            }
        } else if input.chars().nth(i).unwrap() == '"' {
            i += 1;
            while i + 1 < input.len() && input.chars().nth(i).unwrap() != '\'' {
                i += 1;
            }
        } else if i + 1 < input.len() && input.chars().nth(i).unwrap() == '&' {
            commands.push(CommandObject {
                text: input[j..i].trim().to_string(),
                separator: Separator::Ampersand,
                status_code: 0,
            });
            i += 1;
            j = i + 2;
        } else if i + 1 < input.len()
            && input.chars().nth(i).unwrap() == '>'
            && input.chars().nth(i + 1).unwrap() == '>'
        {
            commands.push(CommandObject {
                text: input[j..i].trim().to_string(),
                separator: Separator::WriteAppendRedirection,
                status_code: 0,
            });
            i += 1;
            j = i + 2;
        } else if input.chars().nth(i).unwrap() == '|' {
            commands.push(CommandObject {
                text: input[j..i].trim().to_string(),
                separator: Separator::Pipe,
                status_code: 0,
            });
            i += 1;
            j = i + 1;
        } else if input.chars().nth(i).unwrap() == ';' {
            commands.push(CommandObject {
                text: input[j..i].trim().to_string(),
                separator: Separator::SemiColon,
                status_code: 0,
            });
            i += 1;
            j = i + 1;
        } else if input.chars().nth(i).unwrap() == '>' {
            commands.push(CommandObject {
                text: input[j..i].trim().to_string(),
                separator: Separator::WriteRedirection,
                status_code: 0,
            });
            i += 1;
            j = i + 1;
        }
        i += 1;
    }

    commands.push(CommandObject {
        text: input[j..i].trim().to_string(),
        separator: Separator::Empty,
        status_code: 0,
    });

    return commands;
}
