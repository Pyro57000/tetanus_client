use crate::get_user_input;
use crate::lib::Destination;
use crate::lib::Message;
use crate::lib::Project;
use crate::lib::Table;
use crate::print_error;
use crate::print_success;
use dns_lookup::lookup_host;
use std::collections::HashMap;
use std::fmt::Arguments;
use std::fs::create_dir;
use std::fs::create_dir_all;
use std::fs::{File, OpenOptions, ReadDir, read_to_string};
use std::io::Write;
use std::path::PathBuf;
use std::process::Output;
use std::process::exit;
use std::result;
use std::thread::sleep;
use std::time::Duration;
use tokio;
use tokio::spawn;
use tokio::sync::mpsc::{Receiver, Sender, channel};

#[derive(Clone)]
pub struct ToolCommand {
    pub name: String,
    pub help: String,
    pub req_args: Vec<String>,
    pub user_args: Vec<String>,
    pub console_tx: Sender<Message>,
    pub optional_args: Vec<String>,
    pub args: Option<Vec<ToolArgument>>,
    pub interactive: bool,
    pub optionally_interactive: bool,
    pub func: fn(
        Option<Vec<ToolArgument>>,
        Sender<Message>,
        Option<Sender<Message>>,
        Option<Receiver<Message>>,
    ),
}

impl ToolCommand {
    pub fn new(
        name: String,
        help: String,
        tx: Sender<Message>,
        func: fn(
            Option<Vec<ToolArgument>>,
            Sender<Message>,
            Option<Sender<Message>>,
            Option<Receiver<Message>>,
        ),
    ) -> Self {
        Self {
            name,
            help,
            req_args: Vec::new(),
            user_args: Vec::new(),
            console_tx: tx,
            optional_args: Vec::new(),
            interactive: false,
            optionally_interactive: false,
            args: None,
            func: func,
        }
    }

    pub async fn execute(
        mut self,
        execute_rx: Option<Receiver<Message>>,
        runtime: tokio::runtime::Handle,
    ) {
        if self.interactive {
            let message = Message {
                source: Destination::Console,
                destination: Destination::Control,
                content: String::from("interactive"),
            };
            self.console_tx.send(message).await.unwrap();
            let (command_tx, mut command_rx) = channel(1);
            let console_tx = self.console_tx.clone();
            runtime.spawn_blocking(move || {
                (self.func)(self.args, self.console_tx, Some(command_tx), execute_rx)
            });
            loop {
                let rx_res = command_rx.try_recv();
                if rx_res.is_ok() {
                    let message = rx_res.unwrap();
                    match message.content.as_str() {
                        "noninteractive" => {
                            self.interactive = false;
                            console_tx.send(message).await.unwrap();
                        }
                        "finished" => {
                            console_tx.send(message).await.unwrap();
                            break;
                        }
                        _ => {}
                    }
                }
            }
        } else {
            runtime.spawn(async move { (self.func)(self.args, self.console_tx, None, None) });
        }
    }
}

#[derive(Clone, Default)]
pub struct ToolArgument {
    pub name: String,
    pub user_supplied: bool,
    pub optional: bool,
    pub position: Option<usize>,
    pub path: Option<PathBuf>,
    pub string: Option<String>,
    pub project: Option<Project>,
    pub projects: Option<Vec<Project>>,
    pub boolean: Option<bool>,
    pub console_tx: Option<Sender<String>>,
    pub tx: Option<Sender<String>>,
}

pub fn build_tools(tx: Sender<Message>) -> Vec<ToolCommand> {
    let mut tool_commands = Vec::new();
    let mut list_projects = ToolCommand::new(
        "list_projects".to_string(),
        "lists the currently tracked projects".to_string(),
        tx.clone(),
        list_projects,
    );
    list_projects.req_args = vec![String::from("projects")];
    list_projects.interactive = false;
    tool_commands.push(list_projects);
    let mut new_project_command = ToolCommand::new("new_project".to_string(), "creates a new project. you can give it a project name as an argument, or it will prompt you for the name.".to_string(), tx.clone(), new_project);
    new_project_command.req_args = vec![
        String::from("templatebox"),
        String::from("upcoming_notes"),
        String::from("upcoming_files"),
        String::from("config"),
    ];
    new_project_command.optional_args = vec![String::from("name")];
    new_project_command.optionally_interactive = true;
    new_project_command.req_args = vec![
        String::from("config"),
        String::from("upcoming_files"),
        String::from("upcoming_notes"),
        String::from("templatebox"),
    ];
    tool_commands.push(new_project_command);
    let mut promote_project_command = ToolCommand::new(
        "promote_project".to_string(),
        "promote a project to be promoted from upcoming to curent. Optionally takes a name= argument and a home= argument.".to_string(),
        tx.clone(),
        promote_project,
    );
    promote_project_command.req_args = vec![
        String::from("projects"),
        String::from("current_files"),
        String::from("current_notes"),
        String::from("templatebox"),
    ];
    promote_project_command.optionally_interactive = true;
    promote_project_command.optional_args = vec![String::from("name"), String::from("home")];
    tool_commands.push(promote_project_command);
    return tool_commands;
}

pub fn build_args(
    projects: &Vec<Project>,
    config: &PathBuf,
    settings: &HashMap<String, String>,
) -> Vec<ToolArgument> {
    let mut args = Vec::new();
    for project in projects {
        if project.active {
            let mut project_arg = ToolArgument::default();
            project_arg.name = String::from("active");
            project_arg.project = Some(project.clone());
            args.push(project_arg);
        }
    }
    let mut projects_arg = ToolArgument::default();
    projects_arg.name = String::from("projects");
    projects_arg.projects = Some(projects.clone());
    args.push(projects_arg);
    let mut config_arg = ToolArgument::default();
    config_arg.name = String::from("config");
    config_arg.path = Some(config.clone());
    args.push(config_arg);
    for setting in settings.keys() {
        match setting.as_str() {
            "templatebox" => {
                let mut template_arg = ToolArgument::default();
                template_arg.name = String::from("templatebox");
                let boxname = settings.get("templatebox").unwrap().clone();
                template_arg.string = Some(boxname);
                args.push(template_arg);
            }
            "current_notes" => {
                let mut current_note_arg = ToolArgument::default();
                current_note_arg.name = String::from("current_notes");
                let path = PathBuf::from(settings.get("current_notes").unwrap());
                current_note_arg.path = Some(path);
                args.push(current_note_arg);
            }
            "current_files" => {
                let mut new_arg = ToolArgument::default();
                new_arg.name = String::from("current_files");
                let path = PathBuf::from(settings.get("current_files").unwrap());
                new_arg.path = Some(path);
                args.push(new_arg);
            }
            "upcoming_files" => {
                let mut new_arg = ToolArgument::default();
                new_arg.name = String::from("upcoming_files");
                let path = PathBuf::from(settings.get("upcoming_files").unwrap());
                new_arg.path = Some(path);
                args.push(new_arg);
            }
            "upcoming_notes" => {
                let mut new_arg = ToolArgument::default();
                new_arg.name = String::from("upcoming_notes");
                let path = PathBuf::from(settings.get("upcoming_notes").unwrap());
                new_arg.path = Some(path);
                args.push(new_arg);
            }
            "tools" => {
                let mut new_arg = ToolArgument::default();
                new_arg.name = String::from("tools");
                let path = PathBuf::from(settings.get("tools").unwrap());
                new_arg.path = Some(path);
                args.push(new_arg);
            }
            "distrobox" => {
                let mut new_arg = ToolArgument::default();
                new_arg.name = String::from("distrobox");
                if settings.get("distrobox").unwrap().contains("yes") {
                    new_arg.boolean = Some(true);
                } else {
                    new_arg.boolean = Some(false);
                }
                args.push(new_arg);
            }
            _ => {}
        }
    }
    return args;
}

pub async fn send_command_output(tx: Sender<Message>, message: Message) {
    tx.send(message.clone()).await.unwrap();
}

pub fn list_projects(
    args: Option<Vec<ToolArgument>>,
    tx: Sender<Message>,
    _command_tx: Option<Sender<Message>>,
    _rx: Option<Receiver<Message>>,
) {
    let given_args = args.unwrap();
    let mut lines = vec![String::from("name|stage|boxname")];
    for arg in given_args {
        if arg.name == String::from("projects") {
            for project in arg.projects.unwrap() {
                if project.current {
                    let line = format!("{}|{}|{}", project.name, "current", project.boxname);
                    lines.push(line);
                } else {
                    let line = format!("{}|{}|{}", project.name, "upcomming", project.boxname);
                    lines.push(line);
                }
            }
        }
    }
    let mut table = Table::default();
    table.build(lines);
    let message = Message {
        source: Destination::Console,
        destination: Destination::Console,
        content: table.get_table(),
    };
    tokio::spawn(send_command_output(tx, message.clone()));
}

pub fn new_project(
    args: Option<Vec<ToolArgument>>,
    tx: Sender<Message>,
    comand_tx: Option<Sender<Message>>,
    rx: Option<Receiver<Message>>,
) {
    let mut interactive = false;
    let given_args = args.unwrap();
    let mut config_path = PathBuf::new();
    let mut name = String::new();
    let mut files_path = PathBuf::new();
    let mut notes_path = PathBuf::new();
    let mut template_box = String::new();
    let non_interactive_message = Message {
        source: Destination::Control,
        destination: Destination::Console,
        content: String::from("noninteractive"),
    };
    let mut interactive_message = Message {
        source: Destination::Control,
        destination: Destination::Console,
        content: String::from("interactive"),
    };
    for arg in given_args {
        match arg.name.as_str() {
            "config" => {
                config_path = arg.path.unwrap();
            }
            "name" => {
                name = arg.string.unwrap();
            }
            "upcoming_files" => {
                files_path = arg.path.unwrap();
            }
            "upcoming_notes" => {
                notes_path = arg.path.unwrap();
            }
            "template" => {
                template_box = arg.string.unwrap();
            }
            _ => {}
        }
    }
    if name.len() == 0 {
        interactive = true;
    }
    if interactive {
        let message = Message {
            source: Destination::Console,
            destination: Destination::Console,
            content: String::from("project_name?"),
        };
        let mut rx = initialize_interactive(rx, tx.clone());
        let mut name_asked = false;
        loop {
            if !name_asked {
                tx.blocking_send(message.clone()).unwrap();
            }
            let response = rx.blocking_recv().unwrap();
            name = response.content;
            break;
        }
        comand_tx
            .clone()
            .unwrap()
            .blocking_send(non_interactive_message.clone())
            .unwrap();
    }
    let mut project_path = config_path.clone();
    project_path.pop();
    project_path.push("projects");
    project_path.push(format!("{}.conf", &name));
    let file_create_res = File::create(&project_path);
    if file_create_res.is_err() {
        let message = Message {
            source: Destination::Console,
            destination: Destination::Console,
            content: format!("error! unable to create {}", &project_path.display()),
        };
        tokio::spawn(send_command_output(tx.clone(), message));
        if interactive {
            comand_tx
                .clone()
                .unwrap()
                .blocking_send(non_interactive_message.clone())
                .unwrap();
        }
        return;
    }
    files_path.push(&name);
    notes_path.push(&name);
    let files_dir_res = create_dir_all(&files_path);
    let notes_dir_res = create_dir_all(&notes_path);
    if files_dir_res.is_err() {
        let message = Message {
            source: Destination::Console,
            destination: Destination::Console,
            content: format!(
                "Error failure to create project files folder!\n{}",
                files_dir_res.err().unwrap()
            ),
        };
        tokio::spawn(send_command_output(tx.clone(), message));
        if interactive {
            comand_tx
                .clone()
                .unwrap()
                .blocking_send(non_interactive_message.clone())
                .unwrap();
        }
        return;
    }
    if notes_dir_res.is_err() {
        let message = Message {
            source: Destination::Console,
            destination: Destination::Console,
            content: format!(
                "Error failure to create project files folder!\n{}",
                files_dir_res.err().unwrap()
            ),
        };
        tokio::spawn(send_command_output(tx.clone(), message));
        if interactive {
            comand_tx
                .clone()
                .unwrap()
                .blocking_send(non_interactive_message.clone())
                .unwrap();
        }
        return;
    }
    let mut new_project = Project::default();
    new_project.name = name.clone();
    new_project.files = files_path;
    new_project.notes = notes_path;
    new_project.current = false;
    new_project.boxname = format!("{}_{}", template_box, name);
    new_project.config = project_path;
    println!("{}", new_project.config.display());
    new_project.save_project();
    print_success("folder structure and config file created successfully!");
    println!("setting up default notes...");
    let res = new_project.generate_default_notes(&config_path);
    let message = Message {
        source: Destination::Console,
        destination: Destination::Console,
        content: res,
    };
    tokio::spawn(send_command_output(tx.clone(), message));
    let message = Message {
        source: Destination::Console,
        destination: Destination::Console,
        content: String::from("finished"),
    };
    tokio::spawn(send_command_output(tx.clone(), message));
}

pub fn promote_project(
    args: Option<Vec<ToolArgument>>,
    tx: Sender<Message>,
    comand_tx: Option<Sender<Message>>,
    rx: Option<Receiver<Message>>,
) {
    let mut project = String::new();
    let mut projects = Vec::new();
    let mut files = PathBuf::new();
    let mut notes = PathBuf::new();
    let mut tools = PathBuf::new();
    let mut template = String::new();
    let mut home = None;
    let mut given_args = Vec::new();
    let mut interactive = true;
    if args.is_some() {
        given_args = args.unwrap();
    }
    for arg in given_args {
        match arg.name.as_str() {
            "name" => {
                project = arg.string.clone().unwrap();
                interactive = false;
            }
            "projects" => projects = arg.projects.clone().unwrap(),
            "current_files" => files = arg.path.unwrap(),
            "current_notes" => notes = arg.path.unwrap(),
            "templatebox" => template = arg.string.unwrap(),
            "home" => home = arg.path,
            "tools" => tools = arg.path.unwrap(),
            _ => {}
        }
    }
    if interactive {
        let rx = initialize_interactive(rx, tx.clone());
        let mut lines = vec![String::from("id|name|status")];
        let mut id = 0;
        for project in &projects {
            let mut line = format!("{}|", id);
            id += 1;
            line.push_str(&project.name);
            if project.current {
                line.push_str("|current");
            } else {
                line.push_str("|upcoming");
            }
            lines.push(line);
        }
        let mut project_table = Table::default();
        project_table.build(lines);
        let projects_string = project_table.get_table();
        let table_message = Message {
            source: Destination::Console,
            destination: Destination::Console,
            content: projects_string,
        };
        tx.blocking_send(table_message).unwrap();
        let (gotten_name, mut rx) = prompt_interactive(rx, tx.clone(), "project to promote?");
        let selection: usize = gotten_name.parse().unwrap();
        project = format!("{}", &projects[selection].name);
    }
    for mut existing_project in projects {
        if existing_project.name == project {
            let promote_res = existing_project.promote_project(
                &files,
                &notes,
                template.clone(),
                &tools,
                home.clone(),
            );
            tx.blocking_send(Message {
                source: Destination::Console,
                destination: Destination::Console,
                content: promote_res,
            })
            .unwrap();
        }
    }
    deinitialize_interactive(tx.clone());
}

pub fn remove_project(args: Option<Vec<ToolArgument>>) -> String {
    let mut project = Project::default();
    let mut projects = Vec::new();
    if args.is_some() {
        let given_args = args.unwrap();
        for arg in given_args {
            match arg.name.as_str() {
                "project" => project = arg.project.unwrap(),
                "projects" => projects = arg.projects.unwrap(),
                _ => {}
            }
        }
    }
    if !project.name.len() > 1 {
        let mut current_index = 0;
        for project in &projects {
            println!("{}.) {}", current_index, project.name);
            current_index += 1;
        }
        let response: usize = get_user_input("project to remove?").parse().unwrap();
        if response >= projects.len() {
            return format!("error invalid project selection!");
        }
        project = projects[response].clone()
    }
    return project.remove_project();
}

pub fn subdomain_brute(args: Option<Vec<ToolArgument>>) -> String {
    let mut result_string = String::from("subdmain bruteforce failed!!");
    let mut target_domain = String::new();
    let mut dict = PathBuf::new();
    let mut project = Project::default();
    if args.is_some() {
        let iargs = args.unwrap();
        for arg in iargs {
            if arg.user_supplied {
                match arg.name.as_str() {
                    "wordlist" => dict = PathBuf::from(arg.string.unwrap()),
                    "domain" => target_domain = arg.string.unwrap(),
                    _ => {}
                }
            } else if arg.name == "project" {
                project = arg.project.unwrap();
            }
        }
    }
    if target_domain.len() > 1 {
        target_domain = get_user_input("target domain to bruteforce?");
    }
    if !dict.exists() {
        dict = PathBuf::from(get_user_input("path to wordlist?"));
    }
    print_success("arguments parsed successfully!");
    println!("starting bruteforce...");
    let wordlist_read_res = read_to_string(dict);
    if wordlist_read_res.is_err() {
        print_error(
            "error reading wordlist!",
            Some(wordlist_read_res.err().unwrap().to_string()),
        );
        return result_string;
    }
    let wordlist = wordlist_read_res.unwrap();
    let mut results = Vec::new();
    for sub in wordlist.lines() {
        let mut found_ips = Vec::new();
        let domain_name = format!("{}.{}", sub, target_domain);
        let lookup_res = lookup_host(&domain_name);
        if lookup_res.is_ok() {
            let ips = lookup_res.unwrap();
            for ip in ips {
                found_ips.push(ip);
            }
        }
        if found_ips.len() > 0 {
            let mut ip_string = String::new();
            for ip in found_ips {
                ip_string.push_str(&format!("{},", ip));
            }
            results.push(format!("{} | {}", domain_name, ip_string));
        }
    }
    let mut enumeration_path = project.notes.clone();
    enumeration_path.push("enumeration.md");
    let note_open_res = OpenOptions::new()
        .create_new(true)
        .append(true)
        .open(enumeration_path);
    if note_open_res.is_err() {
        print_error(
            "error opening enumeration notes file!",
            Some(note_open_res.err().unwrap().to_string()),
        );
    } else {
        let mut note_file = note_open_res.unwrap();
        let mut note_text = String::from("\n\n# Subdomain Bruteforce\n");
        note_text.push_str(&format!("## {}", target_domain));
        note_text.push_str("\n| domain name | IP Addresses |\n");
        note_text.push_str("| ----------- | ------------ |\n");
        for res in results {
            note_text.push_str(&format!("| {} |\n", res));
        }
        let write_res = write!(note_file, "{}", note_text);
        if write_res.is_ok() {
            write_res.unwrap();
            result_string = String::from("Subdomain bruteforcing completed successfully!");
            print_success("Subdomain brueforcing completed successfully!");
        }
    }
    return result_string;
}

pub fn activate_project(args: Option<Vec<ToolArgument>>) -> String {
    let mut result_string = String::from("Failed to activeate project!");
    let mut projects = Vec::new();
    let mut activate_target = String::new();
    for arg in args.unwrap() {
        if arg.user_supplied {
            if arg.name == "project" {
                activate_target = arg.string.unwrap();
            }
        }
        match arg.name.as_str() {
            "projects" => projects = arg.projects.unwrap().clone(),
            _ => {}
        }
    }
    for mut project in projects {
        if project.name == activate_target {
            project.active = true;
            result_string = String::from("Project Activated Successfully!");
        } else {
            project.active = false;
        }
        project.save_project();
    }
    return result_string;
}

pub fn initialize_interactive(
    rx: Option<Receiver<Message>>,
    tx: Sender<Message>,
) -> Receiver<Message> {
    let mut rx = rx.unwrap();
    let mut acked = false;
    loop {
        let response = rx.blocking_recv().unwrap();
        match response.content.as_str() {
            "ack" => {
                if !acked {
                    tx.blocking_send(response.clone()).unwrap();
                    acked = true;
                }
            }
            "ready" => {
                return rx;
            }
            _ => {}
        }
    }
}

pub fn prompt_interactive(
    mut rx: Receiver<Message>,
    tx: Sender<Message>,
    prompt: &str,
) -> (String, Receiver<Message>) {
    let message = Message {
        source: Destination::Console,
        destination: Destination::Console,
        content: String::from(prompt),
    };
    tx.blocking_send(message);
    let mut return_string = String::new();
    loop {
        let rx_res = rx.try_recv();
        if rx_res.is_ok() {
            let response = rx_res.unwrap();
            return_string = response.content;
            break;
        }
    }
    return (return_string, rx);
}

pub fn deinitialize_interactive(tx: Sender<Message>) {
    tx.blocking_send(Message {
        source: Destination::Console,
        destination: Destination::Console,
        content: String::from("noninteractive"),
    })
    .unwrap();
}
