use std::io::{self, Write};
use nix::unistd::{fork, execvp, pipe, dup2, close, ForkResult};
use nix::sys::wait::waitpid;
use std::ffi::CString;
use std::fs::File;
use std::os::unix::io::AsRawFd;
use libc;

const MAX_LINE: usize = 80;

fn main() {
    let mut should_run = true;
    let mut history: Vec<String> = Vec::new();

    while should_run {
        print!("osh> ");
        io::stdout().flush().expect("Falha no flush");

        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Falha ao ler entrada");

        let input = input.trim().to_string();

        if input == "exit" {
            should_run = false;
            continue;
        } else if input == "!!" {
            if let Some(last_command) = history.last() {
                println!("{}", last_command);
                history.push(last_command.clone());
            } else {
                eprintln!("Nenhum comando no histÃ³rico.");
                continue;
            }
        } else {
            history.push(input.clone());
        }

        let args: Vec<&str> = input.split_whitespace().collect();

        if args.is_empty() {
            continue;
        }

        if args.contains(&"|") {
            let commands: Vec<&str> = input.split('|').map(|s| s.trim()).collect();
            exe_pipeline(&commands);
        } else if args.contains(&">") || args.contains(&"<") {
            let position = args.iter().position(|&r| r == ">" || r == "<");
            if let Some(pos) = position {
                let (command, file, is_output) = if args[pos] == ">" {
                    (&args[..pos], args[pos + 1], true)
                } else {
                    (&args[..pos], args[pos + 1], false)
                };
                redirection_command(command, file, is_output);
            }
        } else {
            let background = args.last() == Some(&"&");
            let command = if background { &args[..args.len() - 1] } else { &args };
            new_fork(command.to_vec(), background);
        }
    }
}

fn new_fork(args: Vec<&str>, background: bool) {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            let c_args: Vec<CString> = args.iter().map(|&arg| CString::new(arg).unwrap()).collect();
            let c_args_ref: Vec<&CString> = c_args.iter().collect();
            execvp(&c_args_ref[0], &c_args_ref).expect("Falha no execvp");
        }
        Ok(ForkResult::Parent { .. }) => {
            if !background {
                waitpid(None, None).expect("Falha no waitpid");
            }
        }
        Err(_) => eprintln!("Falha no fork"),
    }
}

fn redirection_command(command: &[&str], file: &str, is_output: bool) {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            let fd = if is_output {
                File::create(file).expect("Falha ao abrir arquivo de saÃ­da").as_raw_fd()
            } else {
                File::open(file).expect("Falha ao abrir arquivo de entrada").as_raw_fd()
            };

            dup2(fd, if is_output { libc::STDOUT_FILENO } else { libc::STDIN_FILENO }).expect("Falha no dup2");
            let c_args: Vec<CString> = command.iter().map(|&arg| CString::new(arg).unwrap()).collect();
            let c_args_ref: Vec<&CString> = c_args.iter().collect();
            execvp(&c_args_ref[0], &c_args_ref).expect("Falha no execvp");
        }
        Ok(ForkResult::Parent { .. }) => {
            waitpid(None, None).expect("Falha no waitpid");
        }
        Err(_) => eprintln!("Falha no fork"),
    }
}

fn exe_pipeline(commands: &[&str]) {
    let mut fds = Vec::new();
    for _ in 0..commands.len() - 1 {
        fds.push(pipe().expect("Falha ao criar pipe"));
    }

    for (i, cmd) in commands.iter().enumerate() {
        match unsafe { fork() } {
            Ok(ForkResult::Child) => {
                if i > 0 {
                    close(fds[i - 1].1).expect("Falha ao fechar pipe de escrita");
                    dup2(fds[i - 1].0, libc::STDIN_FILENO).expect("Falha no dup2 para stdin");
                }
                if i < commands.len() - 1 {
                    close(fds[i].0).expect("Falha ao fechar pipe de leitura");
                    dup2(fds[i].1, libc::STDOUT_FILENO).expect("Falha no dup2 para stdout");
                }
                let args: Vec<CString> = cmd.split_whitespace().map(|arg| CString::new(arg).unwrap()).collect();
                let c_args_ref: Vec<&CString> = args.iter().collect();
                execvp(&c_args_ref[0], &c_args_ref).expect("Falha no execvp");
            }
            Ok(ForkResult::Parent { .. }) => {
                if i > 0 {
                    close(fds[i - 1].0).expect("Falha ao fechar pipe de leitura no pai");
                    close(fds[i - 1].1).expect("Falha ao fechar pipe de escrita no pai");
                }
                waitpid(None, None).expect("Falha no waitpid");
            }
            Err(_) => eprintln!("Falha no fork"),
        }
    }
}
