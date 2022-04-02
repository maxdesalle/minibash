use std::collections::HashMap;
use std::io::stdin;
use std::io::stdout;
use std::io::Write;

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
        let mut iterator = commands.iter();

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

            let mut args = splitter(&dollar_expander(&mut env, command.text.clone()));
            if args.is_empty() {
                continue;
            }

            let mut status_code = 0;
            command_matcher(&mut env, &mut args, &mut status_code);

            if status_code != 0 && command.separator == Separator::Ampersand {
                skip_until_semicolon = true;
            }
        }
    }
}
