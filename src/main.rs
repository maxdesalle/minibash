use std::collections::HashMap;
use std::fs::File;
use std::io::stdin;
use std::io::stdout;
use std::io::Write;
use std::os::unix::io::{FromRawFd, IntoRawFd};
use std::process::{Child, Stdio};

pub mod lib;
use lib::*;

fn main() {
    let mut env: HashMap<String, String> = save_env();
    update_shlvl(&mut env);

    loop {
        unsafe {
            libc::signal(libc::SIGINT, handle_sigint as libc::sighandler_t);
            libc::signal(libc::SIGUSR1, handle_sigusr1 as libc::sighandler_t);
            libc::signal(libc::SIGQUIT, handle_sigquit as libc::sighandler_t);
        }

        let mut skip_until_semicolon = false;

        print!("minibash-3.2$ ");
        if let Err(error) = stdout().flush() {
            eprintln!("{}", error);
        }

        let mut input = String::new();
        stdin().read_line(&mut input).unwrap();
        if input.is_empty() {
            unsafe {
                handle_sigusr1(libc::SIGUSR1);
            }
        }
        let mut input = input.trim().to_string();
        let commands: Vec<CommandObject> = arg_split(&mut input);
        let mut iterator = commands.iter().peekable();
        let mut skip = false;

        while let Some(command) = iterator.next() {
            if skip_until_semicolon == true {
                if command.separator != Separator::SemiColon
                    && command.separator != Separator::Empty
                {
                    continue;
                } else {
                    skip_until_semicolon = false;
                    if command.separator == Separator::Empty {
                        break;
                    } else {
                        continue;
                    }
                }
            }

            if skip == true {
                skip = false;
                continue;
            }

            let input_output =
                if command.separator == Separator::WriteRedirection && iterator.peek().is_some() {
                    skip = true;
                    let file = File::create(iterator.peek().unwrap().text.clone());
                    match file {
                        Ok(file) => {
                            let file_out = file.try_clone();
                            unsafe {
                                Some(InputOutput {
                                    file: Some(file),
                                    stdout: Stdio::from_raw_fd(file_out.unwrap().into_raw_fd()),
                                    output: None,
                                })
                            }
                        }
                        Err(_) => None,
                    }
                } else if command.separator == Separator::Pipe && iterator.peek().is_some() {
                    Some(InputOutput {
                        file: None,
                        stdout: Stdio::piped(),
                        output: None,
                    })
                } else {
                    Some(InputOutput {
                        file: None,
                        stdout: Stdio::inherit(),
                        output: None,
                    })
                };

            let mut args = splitter(&dollar_expander(&mut env, command.text.clone()));
            if args.is_empty() {
                continue;
            }

            let mut command = command.clone();

            match input_output {
                Some(input_output) => {
                    command_matcher(&mut env, &mut args, &mut command, input_output);
                }
                None => {
                    skip_until_semicolon = true;
                    eprintln!("Error opening file");
                    continue;
                }
            };

            if command.status_code != 0 && command.separator == Separator::Ampersand {
                skip_until_semicolon = true;
            }
        }
    }
}
