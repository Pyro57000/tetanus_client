use crate::commands;
use crate::commands::ToolArgument;
use crate::commands::ToolCommand;
use crate::lib::Message;
use crate::load_projects;
use crate::load_settings;
use crate::network;
use crate::print_error;
use crate::{get_user_input, lib::Destination, lib::Project, lib::Table, print_success};
use std::io::Read;
use std::path::PathBuf;
use tokio;
use tokio::sync::mpsc::{Receiver, Sender, channel};

pub async fn rec_message(mut rx: Receiver<Message>) {
    let mut display = true;
    loop {
        let rx_res = rx.try_recv();
        if rx_res.is_ok() {
            let message = rx_res.unwrap();
            if message.content.to_lowercase().contains("error") {
                print_error(&message.content, None);
            } else {
                print_success(&message.content);
            }
            display = true;
        }
        if display {
            println!("\n\ncommand?");
            display = false;
        }
    }
}

pub async fn cli(
    mut projects: Vec<Project>,
    main_tx: Sender<Message>,
    tool_tx: Sender<Message>,
    server_address: String,
    config_path: PathBuf,
) {
    print_success("started the CLI!");
    let mut commands = commands::build_tools();
    loop {
        let mut valid_command = false;
        let settings = load_settings(&config_path, false);
        projects = load_projects(&config_path, false);
        if tool_tx.is_closed() {
            println!("tool tx closed at the start of this loop!");
        }
        let command = get_user_input("");
        let mut command_name = String::new();
        let mut args = Vec::new();
        if command.contains(" ") {
            let mut command_vec: Vec<&str> = command.split(" ").collect();
            command_name = command_vec[0].to_string();
            for arg in &mut command_vec[1..] {
                args.push(arg.to_string());
            }
        } else {
            command_name = command;
        }
        if command_name == String::from("exit") {
            let message = Message {
                source: Destination::Console,
                destination: Destination::Control,
                content: String::from("exit"),
            };
            valid_command = true;
            main_tx.send(message).await.unwrap();
        }
        for toolcommand in &mut commands {
            let mut ready = true;
            if toolcommand.name == command_name {
                valid_command = true;
                if toolcommand.req_args.len() > 0 {
                    let mut args_vec = Vec::new();
                    for req_arg in toolcommand.req_args.clone() {
                        let mut new_arg = ToolArgument::default();
                        new_arg.name = req_arg;
                        args_vec.push(new_arg);
                    }
                    if args.len() < toolcommand.user_args.len() {
                        ready = false;
                        print_error("not enough arguments provided!", None);
                        let mut command_usage = format!("{}", toolcommand.name.clone());
                        for arg in toolcommand.user_args.clone() {
                            command_usage.push_str(&format!(" {}", &arg));
                        }
                        print_error("usage:", Some(command_usage));
                    }
                    if ready {
                        let mut position_count = 0;
                        for user_arg in toolcommand.user_args.clone() {
                            println!("enough args supplied, building arguments list");
                            let mut new_arg = ToolArgument::default();
                            new_arg.name = user_arg;
                            new_arg.user_supplied = true;
                            new_arg.position = Some(position_count);
                            args_vec.push(new_arg);
                            position_count += 1;
                        }
                    }
                    if ready {
                        for arg in &mut args_vec {
                            let iargs = args.clone();
                            if arg.user_supplied {
                                arg.string = Some(iargs[arg.position.unwrap()].clone());
                            } else {
                                match arg.name.as_str() {
                                    "projects" => arg.projects = Some(projects.clone()),
                                    "upcoming_files" => {
                                        arg.path =
                                            Some(PathBuf::from(settings["upcoming_files"].clone()))
                                    }
                                    "upcoming_notes" => {
                                        arg.path =
                                            Some(PathBuf::from(settings["upcoming_notes"].clone()))
                                    }
                                    "template" => {
                                        arg.string =
                                            Some(String::from(settings["templatebox"].clone()))
                                    }
                                    "config" => arg.path = Some(config_path.clone()),
                                    _ => print_error(
                                        &format!("unkown arg requested! {}", arg.name),
                                        None,
                                    ),
                                }
                            }
                        }
                        toolcommand.args = Some(args_vec);
                    }
                }
                let mut message = Message {
                    source: Destination::Console,
                    destination: Destination::Console,
                    content: String::new(),
                };
                if ready {
                    message.content = toolcommand.execute();
                } else {
                    message.content = String::from("error in command!");
                }
                tool_tx.send(message).await.unwrap();
            }
        }
        if !valid_command {
            let message = Message {
                source: Destination::Console,
                destination: Destination::Console,
                content: String::from("error: command not found!"),
            };
            tool_tx.send(message).await.unwrap();
        }
    }
}
