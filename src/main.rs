use chacha20poly1305::aead::generic_array::GenericArray;
use chacha20poly1305::aead::generic_array::typenum::Unsigned;
use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::{ChaCha20Poly1305, Key};
use clap::Parser;
use colored::Colorize;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions, read_dir, read_to_string};
use std::io::{Read, Write};
use std::os::unix::net;
use std::path::{Display, PathBuf};
use std::process::{Output, exit};
use std::thread::sleep;
use std::time::Duration;
use std::{thread, time};
use tokio;
use tokio::sync::mpsc::{Receiver, Sender, channel};

mod cli;
mod commands;
mod crytpo;
mod install;
mod lib;
mod network;

#[derive(Debug, Parser)]
#[command(
    version,
    about,
    long_about = "The server part of tetanus! It will read the config and load up any known clients and stuff."
)]
struct Args {
    #[arg(
        short,
        long,
        help = "the server to connect to, defaults to 127.0.0.1:31337"
    )]
    server: Option<String>,

    #[arg(short, long, help = "launch in gui mode")]
    gui: bool,

    #[arg(
        short,
        long,
        help = "a path to a custom config file, defaults to ~/.config/tetanus/clients/main_attacker.conf"
    )]
    config: Option<PathBuf>,

    #[arg(short, long, help = "generate or re-generate the config file")]
    install: bool,

    #[arg(short, long, help = "custom name to give this client.")]
    name: Option<String>,
}

pub fn print_success(text: &str) {
    println!("{}", text.green());
}

pub fn print_error(text: &str, error: Option<String>) {
    println!("{}", text.red());
    if error.is_some() {
        println!("{}", error.unwrap().red());
    }
}

pub fn get_user_input(prompt: &str) -> String {
    let mut response = String::new();
    loop {
        println!("{}", prompt);
        let res = std::io::stdin().read_line(&mut response);
        if res.is_err() {
            print_error("we need input here dummy, try again...", None);
        } else {
            break;
        }
    }
    return response.trim().to_string();
}

pub fn load_projects(path: &PathBuf, display: bool) -> Vec<lib::Project> {
    let mut projects_path = path.clone();
    projects_path.pop();
    projects_path.push("projects");
    let project_dir_res = read_dir(projects_path);
    if project_dir_res.is_err() {
        print_error(
            "error reading projects directory!",
            Some(project_dir_res.err().unwrap().to_string()),
        );
        exit(1);
    }
    let project_dir = project_dir_res.unwrap();
    let mut projects = Vec::new();
    for res in project_dir {
        if res.is_ok() {
            let mut new_project = lib::Project::default();
            let entry = res.unwrap();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if file_name.contains(".conf") {
                let conf_string_res = read_to_string(entry.path());
                if conf_string_res.is_ok() {
                    let conf_string = conf_string_res.unwrap();
                    for line in conf_string.lines() {
                        let line_vec: Vec<&str> = line.split("|").collect();
                        match line_vec[0] {
                            "name" => {
                                new_project.name = line_vec[1].trim().to_string();
                            }
                            "stage" => {
                                if line_vec[1].contains("current") {
                                    new_project.current = true;
                                } else {
                                    new_project.current = false;
                                }
                            }
                            "files" => {
                                new_project.files = PathBuf::from(line_vec[1]);
                            }
                            "notes" => {
                                new_project.notes = PathBuf::from(line_vec[1]);
                            }
                            "boxname" => new_project.boxname = String::from(line_vec[1]),
                            _ => {
                                print_error(
                                    "unknown setting discoverd in project config file!",
                                    None,
                                );
                            }
                        }
                    }
                    if new_project.boxname.len() > 0 {
                        projects.push(new_project);
                    } else {
                        new_project.boxname = String::from("none");
                        projects.push(new_project);
                    }
                    if display {
                        print_success(
                            format!(
                                "{} successfully loaded!",
                                entry.file_name().to_string_lossy()
                            )
                            .as_str(),
                        );
                    }
                }
            }
        }
    }
    return projects;
}

pub fn save_project(project: &lib::Project, config_path: &PathBuf) {
    let mut conf_open_options = OpenOptions::new();
    if config_path.exists() {
        conf_open_options.append(true);
    } else {
        conf_open_options.create(true);
    }
    let conf_open_create_res = conf_open_options.open(config_path);
    if conf_open_create_res.is_err() {
        print_error(
            "error opening project config path!",
            Some(format!("{}", conf_open_create_res.err().unwrap())),
        );
        return;
    }
    let mut conf_file = conf_open_create_res.unwrap();
    let config_string = format!(
        "name|{}\nstage|upcoming\nfiles|{}\nnotes|{}\nboxname|{}",
        project.name,
        project.files.display(),
        project.notes.display(),
        project.boxname
    );
    write!(conf_file, "{}", config_string).unwrap();
    print_success("project saved!");
}

pub fn load_settings(config_path: &PathBuf, display: bool) -> HashMap<String, String> {
    let mut settings = HashMap::new();
    let conf_read_res = read_to_string(&config_path);
    if conf_read_res.is_err() {
        print_error(
            "error reading config file!",
            Some(conf_read_res.err().unwrap().to_string()),
        );
        exit(1);
    }
    let conf_string = conf_read_res.unwrap();
    if display {
        println!("loading settings from config line...");
    }
    for line in conf_string.lines() {
        if line.contains("|") {
            let line_vec: Vec<&str> = line.split("|").collect();
            settings.insert(line_vec[0].to_string(), line_vec[1].to_string());
            if display {
                print_success(format!("{} {} LOADED!", line_vec[0], line_vec[1]).as_str());
            }
        }
    }
    return settings;
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let mut settings = HashMap::new();
    let mut config_path = PathBuf::new();
    let mut server_address = String::from("127.0.0.1:31337");
    if args.config.is_some() {
        config_path = args.config.unwrap();
    } else {
        let home_res = std::env::home_dir();
        if home_res.is_some() {
            config_path = home_res.unwrap();
            config_path.push(".config/tetanus/clients/main_attacker/config.conf");
        } else {
            print_error(
                "error finding config file!\nplease re-run while specifying a config file",
                None,
            );
        }
    }
    if args.install {
        let res = install::install(&config_path);
        if res {
            print_success("client successfully installed!");
            print_success("please re-run this tool to use it!");
            exit(0);
        }
    } else if !config_path.exists() {
        println!("ooof no config file exitst at {}", config_path.display());
        if get_user_input("would you like to create one?")
            .to_lowercase()
            .contains("y")
        {
            let status = install::install(&config_path);
            if !status {
                print_error("error installing...", None);
                exit(1);
            } else {
                print_success("client successfully installed!");
                print_success("please re-run this tool to use it!");
                exit(0);
            }
        } else {
            print_error("config file does not exist", None);
            exit(1);
        }
    }

    let conf_read_res = read_to_string(&config_path);
    if conf_read_res.is_err() {
        print_error(
            "error reading config file!",
            Some(conf_read_res.err().unwrap().to_string()),
        );
        exit(1);
    }
    let conf_string = conf_read_res.unwrap();
    println!("loading settings from config line...");
    for line in conf_string.lines() {
        if line.contains("|") {
            let line_vec: Vec<&str> = line.split("|").collect();
            settings.insert(line_vec[0].to_string(), line_vec[1].to_string());
            print_success(format!("{} {} LOADED!", line_vec[0], line_vec[1]).as_str());
        }
    }
    let key_path = PathBuf::from(settings["key_file"].clone());
    let mut key_vec = Vec::new();
    let key_open_res = OpenOptions::new().read(true).open(key_path);
    if key_open_res.is_err() {
        print_error(
            "error opening key file!",
            Some(key_open_res.err().unwrap().to_string()),
        );
        exit(1);
    }
    let mut key_file = key_open_res.unwrap();
    let key_read_res = key_file.read(&mut key_vec);
    if key_read_res.is_err() {
        print_error(
            "error reading key",
            Some(key_read_res.err().unwrap().to_string()),
        );
        exit(1);
    }
    key_read_res.unwrap();

    let projects = load_projects(&config_path, true);
    let mut server_address = String::from("127.0.0.1:31337");
    if args.server.is_some() {
        server_address = args.server.unwrap();
    }
    let (main_tx, mut main_rx) = channel(1024);
    let (console_tx, console_rx) = channel(1024);
    if !args.gui {
        let input_handle = tokio::spawn(cli::cli(
            projects,
            main_tx.clone(),
            console_tx.clone(),
            server_address,
            config_path,
        ));
        thread::sleep(Duration::from_secs(1));
        let output_handle = tokio::spawn(cli::rec_message(console_rx));
        loop {
            sleep(Duration::from_secs(1));
            let rx_rex = main_rx.try_recv();
            if rx_rex.is_ok() {
                let message = rx_rex.unwrap();
                if message.destination == lib::Destination::Control {
                    match message.content.as_str() {
                        "exit" => {
                            input_handle.abort();
                            output_handle.abort();
                            exit(0);
                        }
                        _ => {
                            println!("unknown message recieved!");
                            println!("{}", message.content);
                        }
                    }
                }
            }
        }
    } else {
        println!("gui coming soon!");
    }
}
