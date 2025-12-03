use crate::{
    commands::{ToolArgument, build_args, build_tools},
    lib::{Destination, Message, Project},
    load_projects, load_settings, print_error, print_success,
};
use std::{path::PathBuf, thread::sleep, time::Duration};
use tokio::{
    self,
    io::AsyncBufReadExt,
    sync::mpsc::{Receiver, Sender, channel},
};

pub async fn rec_message(mut console_rx: Receiver<Message>, cli_tx: Sender<Message>) {
    print_success("Console output initialized!");
    let prompt = String::from("what is thy bidding my master?");
    let prompt_message = Message {
        source: Destination::Console,
        destination: Destination::Console,
        content: String::from("PROMPT"),
    };
    let mut display = true;
    let mut exit = false;
    let mut interactive = false;
    let mut acked = false;
    loop {
        let mut output = String::new();
        let rx_res = console_rx.try_recv();
        if rx_res.is_ok() {
            let message = rx_res.unwrap();
            if message.source == Destination::Control {
                match message.content.as_str() {
                    "init" => {
                        sleep(Duration::from_secs(5));
                        output.push_str("\nconsole output started successfully!\n");
                        display = true;
                    }
                    "exit" => {
                        output.push_str("\ngood bye!\n");
                        display = true;
                        exit = true;
                    }
                    "status" => {
                        if console_rx.is_closed() {
                            output.push_str("\nerror! the console_rx channel is closed!!!\n");
                            display = true;
                        } else {
                            output.push_str("\nthe console_rx channel is still open!!\n");
                            display = true;
                        }
                    }
                    "noninteractive" => {
                        println!("we got the non-interactive message!");
                        interactive = false;
                        display = true;
                        let done_message = Message {
                            source: Destination::Console,
                            destination: Destination::Console,
                            content: String::from("DONE"),
                        };
                        cli_tx.send(done_message).await.unwrap();
                    }
                    "ack" => {
                        display = false;
                        interactive = true;
                        cli_tx.send(message.clone()).await.unwrap();
                        acked = true;
                    }
                    _ => {
                        output.push_str(&format!("{}", message.content));
                        display = true;
                    }
                }
            } else if message.source == Destination::Server {
                output.push_str(&format!(
                    "\n{} recieved from server! (functionality coming soon!)\n",
                    message.content
                ));
                display = true;
            } else if message.source == Destination::Console {
                output.push_str(&format!("\n{}\n", &message.content));
                display = true;
            }
        }
        if display {
            match interactive {
                false => {
                    if !exit {
                        output.push_str(&prompt);
                        if output.contains("error") {
                            print_error(&output, None);
                        } else {
                            print_success(&output);
                        }
                    }
                }
                true => {
                    if acked {
                        cli_tx.send(prompt_message.clone()).await.unwrap();
                        println!("{}", &output);
                    }
                }
            }
        }
        display = false;
        if exit {
            break;
        }
    }
}

pub async fn console_user_input() -> String {
    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut response = Vec::new();
    let _fut = reader.read_until(b'\n', &mut response).await.unwrap();
    let user_input = match str::from_utf8(&response) {
        Ok(s) => s.to_string(),
        Err(_e) => "".to_string(),
    };
    return user_input;
}

pub async fn cli(
    console_tx: Sender<Message>,
    console_rx: Receiver<Message>,
    main_tx: Sender<Message>,
    config: PathBuf,
    runtime: tokio::runtime::Handle,
) {
    let (cli_tx, mut cli_rx) = channel(1);
    let handle = runtime.spawn(rec_message(console_rx, cli_tx));
    print_success("started the CLI!");
    print_success("happy hacking!");
    loop {
        let input_handle = tokio::spawn(console_user_input());
        let mut settings = load_settings(&config, false);
        let mut projects = load_projects(&config, false);
        let tool_args = build_args(&projects, &config, &settings);
        let tool_commands = build_tools(console_tx.clone());
        let user_input = input_handle.await.unwrap().trim().to_string();
        let mut user_command_name = String::new();
        let mut user_command_args = Vec::new();
        if user_input.contains(" ") {
            let input_vec: Vec<&str> = user_input.split(" ").collect();
            user_command_name = input_vec[0].to_string();
            user_command_args = input_vec[1..].to_vec();
        } else {
            user_command_name = user_input;
        }
        if user_command_name == String::from("exit") {
            let message = Message {
                source: Destination::Control,
                destination: Destination::Console,
                content: String::from("exit"),
            };
            console_tx.send(message).await.unwrap();
            handle.abort();
            let message = Message {
                source: Destination::Console,
                destination: Destination::Control,
                content: String::from("exit"),
            };
            main_tx.send(message).await.unwrap();
            break;
        }
        let mut valid_command = false;
        let mut command_to_run = tool_commands[0].clone();
        for command in &tool_commands {
            if command.name == user_command_name {
                valid_command = true;
                command_to_run = command.clone();
            }
        }
        if valid_command == false {
            let message = Message {
                source: Destination::Console,
                destination: Destination::Console,
                content: String::from("error! invalid command!"),
            };
            console_tx.send(message).await.unwrap();
            continue;
        }
        let mut command_to_run_arg_vec = Vec::new();
        for arg in tool_args {
            if command_to_run.req_args.contains(&arg.name) {
                command_to_run_arg_vec.push(arg.clone());
                println!("{} added!", &arg.name);
            }
        }
        let mut correct_args = false;
        for arg in user_command_args.clone() {
            if !arg.contains("=") {
                let mut new_arg = ToolArgument::default();
                new_arg.name = String::from(arg);
                new_arg.string = Some(String::from(arg));
                command_to_run_arg_vec.push(new_arg);
            }
        }
        if command_to_run_arg_vec.len()
            == command_to_run.req_args.len() + command_to_run.user_args.len()
        {
            correct_args = true;
        }
        for arg in &user_command_args {
            if arg.contains("=") {
                let arg_vec: Vec<&str> = arg.split("=").collect();
                let mut new_arg = ToolArgument::default();
                new_arg.name = String::from(arg_vec[0]);
                new_arg.string = Some(String::from(arg_vec[1]));
                command_to_run_arg_vec.push(new_arg);
            }
        }
        if correct_args == false {
            println!("{}", command_to_run.help);
            print_error(
                "wrong number of arguments supplied!\n please read the above help message.",
                None,
            );
            print_error(
                "",
                Some(format!(
                    "args reqired: {}\nargs given: {}",
                    command_to_run.req_args.len() + command_to_run.user_args.len(),
                    command_to_run_arg_vec.len()
                )),
            );
            continue;
        }
        if command_to_run_arg_vec.len() > 0 {
            command_to_run.args = Some(command_to_run_arg_vec.clone());
        }
        if command_to_run.optionally_interactive {
            if command_to_run_arg_vec.len()
                == command_to_run.req_args.len()
                    + command_to_run.user_args.len()
                    + command_to_run.optional_args.len()
            {
                command_to_run.interactive = false;
            } else {
                command_to_run.interactive = true;
            }
        }
        if command_to_run.interactive {
            println!("we got to the interactive section!");
            let (command_tx, command_rx) = channel(1);
            runtime.spawn(command_to_run.execute(Some(command_rx), runtime.clone()));
            let init_message = Message {
                source: Destination::Control,
                destination: Destination::Console,
                content: String::from("ack"),
            };
            let mut inited = false;
            loop {
                if !inited {
                    command_tx.send(init_message.clone()).await.unwrap();
                }
                let rx_res = cli_rx.try_recv();
                if rx_res.is_ok() {
                    let message = rx_res.unwrap();
                    match message.content.as_str() {
                        "ack" => {
                            inited = true;
                            command_tx
                                .send(Message {
                                    source: Destination::Console,
                                    destination: Destination::Console,
                                    content: String::from("ready"),
                                })
                                .await
                                .unwrap();
                        }
                        "PROMPT" => {
                            let input_handle = tokio::spawn(console_user_input());
                            let response = input_handle.await.unwrap();
                            command_tx
                                .send(Message {
                                    source: Destination::Console,
                                    destination: Destination::Console,
                                    content: response.trim().to_string(),
                                })
                                .await
                                .unwrap();
                        }
                        "DONE" => {
                            break;
                        }
                        _ => {}
                    }
                }
            }
        } else {
            runtime.spawn(command_to_run.execute(None, runtime.clone()));
        }
    }
}
