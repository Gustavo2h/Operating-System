use std::io::{self,Write};
use std::process::{Command, Stdio};
use nix::unistd::{fork, ForkResult};
use nix::sys::wait::wait;

use libc;

use std::fs::File;
use std::os::unix::io::AsRawFd;
use nix::unistd::dup2;

use nix::unistd::{pipe, close};

const MAX_LINE: usize = 80;

fn main() {
	let mut should_run = true;
	let mut args: Vec<&str> = Vec::with_capacity(MAX_LINE/2 + 1);	
	let mut history : Vec<String> = Vec::new();

	while should_run{

		print!("osh> ");
		io::stdout().flush().expect("Falha no flush");

		let mut input = String::new();
		io::stdin().read_line(&mut input).expect("Falha ao ler");
		
		let mut input = input.trim();

		//arrumar o uso das funcoes
		//talvez sumir com a funcao historySearch
		if input=="exit" {
			should_run = false;
			continue;
		}else if input == "!!"{
			if historySearch(&history) == None{
				continue;
			}else{
				input = &historySearch(&history).expect("falha no historico").to_string();
			}
		}else{
			history.push(input.to_string());
		}

		for (i, arg) in input.split_whitespace().enumerate(){
			if i < (MAX_LINE/2 + 1){
				args.push(arg);
			}else{
				eprintln!("Maximo de argumentos excedido");
				break;
			}
		}

		if args.is_empty(){
			continue;
		}

		if args.contains (&"|"){
			let pipe_position = args.iter().position(|&r| r == "|").unwrap();
			let cmd1 = &args[..pipe_position];
			let cmd2 = &args[pipe_position+1..];
			exePipe(cmd1, cmd2);
		}else if args.contains(&">") || args.contains(&"<") {
			let (command, file, isout);
			
			if let Some(position) = args.iter().position(|&r| r == ">"){
				command = &args[..position];
				file = &args[position+1];
				isout = true;
			}else if let Some(position) = args.iter().position(|&r| r=="<"){
				command = &args[..position];
				file = &args[position+1];
				isout = false;
			}else{
				command = &args[..];
				file = &"";
				isout = true;
			};
			redirectionCommand(command,
			if isout {None} else {Some(&file)}, 
			if isout {Some(&file)} else {None});
		}else {
			let backg = args.last() == Some(&"&");
			if backg {
				args.pop();
			}
			newFork(args.clone(),backg);
		}
	}
}

fn newFork (args: Vec<&str>, backg: bool){

	match unsafe {fork()} {
		Ok (ForkResult::Child) => {
			let exe_result = Command::new(args[0])
			.args(&args[1..])
			.stdin(Stdio::inherit())
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.spawn();
			if let Err(e) = exe_result{
				eprintln!("Erro no Child: {}", e);
				std::process::exit(1);
			}
		}
		Ok (ForkResult::Parent { .. }) => {
			if !backg{
				wait().expect("Falha no Wait");
			}
		}
		Err(_)=> eprintln!("Falha no fork"),
	}
}


fn historySearch (history: &[String]) -> Option<String> {
	if let Some(lcommand) = history.last(){
		println!("{}", lcommand);
		Some(lcommand.clone())
	}else{
		eprintln!("No commands in history.");
		None
	}

}

fn redirectionCommand (args: &[&str], input: Option<&str>, output: Option<&str>){

	match unsafe {fork()}{
		Ok(ForkResult::Child)=>{
			if let Some(inputf) = input{
				let file = File::open(inputf).expect("Falha ao abrir input");
				dup2(file.as_raw_fd(), libc::STDIN_FILENO).expect("Falha no dup2");
			}
			if let Some(outputf) = output{
				let file = File::create(outputf).expect("Falha ao abrir output");
				dup2(file.as_raw_fd(), libc::STDOUT_FILENO).expect("Falha no dup2");
			}
			let exe_result = Command::new(args[0])
			.args(&args[1..])
			.stdin(Stdio::inherit())
			.stdout(Stdio::inherit())
			.stderr(Stdio::inherit())
			.spawn();
			if let Err(e) = exe_result{
				eprintln!("Erro: {}", e);
				std::process::exit(1);
			}
		}
		Ok (ForkResult::Parent { .. })=>{
			wait().expect("Falha no wait");
		}
		Err(_) => eprintln!("Falha no fork"),
	}
}

fn exePipe (cmd1:&[&str], cmd2:&[&str]){

	let (pipe_read, pipe_write) = pipe().expect("Falha ao criar pipe");
	
	match unsafe {fork()} {
		Ok(ForkResult::Child) => {
			match unsafe {fork()}{
				Ok(ForkResult::Child) =>{
					close(pipe_read.as_raw_fd()).expect("Falha ao fechar pipe_read");
					dup2(pipe_write, libc::STDOUT_FILENO)
					.expect("Falha no dup2");
					Command::new(cmd1[0])
					.args(&cmd1[1..])
					.spawn()
					.expect("Falha no cmd1");
				}
				Ok(ForkResult::Parent{ .. }) => {
					close(pipe_write.as_raw_fd()).expect("Falha ao fechar pipe_write");
					dup2(pipe_read, libc::STDIN_FILENO)
					.expect("Falha no dup2");
					Command::new(cmd2[0])
					.args(&cmd2[1..])
					.spawn()
					.expect("Falha no cmd2");
				}
			Err(_) => eprintln!("Falha no fork de dentro"),
			}
		}
		Ok(ForkResult::Parent{..}) => {
			wait().expect("Falha no wait");
		}
	Err(_) => eprintln!("Falha no fork de fora"),
	}
}
