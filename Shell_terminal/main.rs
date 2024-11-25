use std::io::{self, Write};
use nix::unistd::{fork, execvp, dup2, pipe, close, ForkResult};
use nix::sys::wait::waitpid;
use nix::errno::Errno;
use std::ffi::CString;
use libc;

fn main() {
    let mut should_run = true;
    let mut history: Vec<String> = Vec::new(); //historico

    while should_run {
        print!("osh> ");
        io::stdout().flush().expect("Falha no flush");

        let mut input = String::new(); //string de entrada
        io::stdin().read_line(&mut input).expect("Falha ao ler entrada");

        let mut input = input.trim().to_string(); // remove os espaços em branco (inicio e fim)

        // limita entrada a 80 caracteres
        if input.len() > 80 {
            input.truncate(80);
            eprintln!("Entrada maior que 80 caracteres.");
        }

        if input == "exit" {
            should_run = false;
            continue;
        } else if input == "!!" {
            if let Some(last_command) = history.last() { //verificação do historico
                println!("{}", last_command);
                input = last_command.clone();
            } else {
                eprintln!("Nenhum comando no histórico.");
                continue;
            }
        }

        history.push(input.clone()); //atualiza o historico

        let args: Vec<&str> = input.split_whitespace().collect(); //criando vetor de argumentos pra usar o execvp

        if args.is_empty() {
            continue;
        }

        let background = args.last() == Some(&"&"); //background = 1 se tiver '&'
        let command = if background { &args[..args.len() - 1] } else { &args };

        if args.contains(&"|") { //caso do pipe
            let commands: Vec<&str> = input.split('|').map(|s| s.trim()).collect();
            exe_pipeline(&commands, background);
        } else if args.contains(&">") || args.contains(&"<") { //caso do redirecionamento
            let position = args.iter().position(|&r| r == ">" || r == "<");
            if let Some(pos) = position {
                let (cmd, file, is_output) = if args[pos] == ">" {
                    (&command[..pos], args[pos + 1], true)
                } else {
                    (&command[..pos], args[pos + 1], false)
                };
                redirection_command(cmd, file, is_output, background);
            }
        } else {
            new_fork(command.to_vec(), background); //comando basico
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

fn redirection_command(command: &[&str], file: &str, is_output: bool, background: bool) {
    match unsafe { fork() } {
        Ok(ForkResult::Child) => {
            let fd = if is_output {
                unsafe { libc::open(CString::new(file).unwrap().as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644) }
            } else {
                unsafe { libc::open(CString::new(file).unwrap().as_ptr(), libc::O_RDONLY) }
            };

            if fd < 0 {
                eprintln!("Erro ao abrir arquivo");
                std::process::exit(1);
            }

            dup2(fd, if is_output { libc::STDOUT_FILENO } else { libc::STDIN_FILENO }).expect("Falha no dup2");
            unsafe { close(fd) };

            let c_args: Vec<CString> = command.iter().map(|&arg| CString::new(arg).unwrap()).collect();
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

fn exe_pipeline(commands: &[&str], background: bool) {
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
                if !background {
                    waitpid(None, None).expect("Falha no waitpid");
                }
            }
            Err(_) => eprintln!("Falha no fork"),
        }
    }
}
