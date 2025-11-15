use crate::get_user_input;
use crate::lib::Destination;
use crate::lib::Message;
use crate::lib::Project;
use crate::lib::Table;
use crate::print_error;
use crate::print_success;
use std::fs::create_dir;
use std::fs::create_dir_all;
use std::fs::{File, OpenOptions, ReadDir, read_to_string};
use std::io::Write;
use std::path::PathBuf;
use std::process::exit;
use tokio;
use tokio::sync::mpsc::{Receiver, Sender};

#[derive(Clone)]
pub struct ToolCommand {
    pub name: String,
    pub help: String,
    pub req_args: Vec<String>,
    pub user_args: Vec<String>,
    pub optional_args: Vec<String>,
    pub args: Option<Vec<ToolArgument>>,
    pub func: fn(Option<Vec<ToolArgument>>) -> String,
}

impl ToolCommand {
    pub fn new(name: String, help: String, func: fn(Option<Vec<ToolArgument>>) -> String) -> Self {
        Self {
            name,
            help,
            req_args: Vec::new(),
            user_args: Vec::new(),
            optional_args: Vec::new(),
            args: None,
            func,
        }
    }

    pub fn execute(&self) -> String {
        if self.req_args.len() > 0 {
            if self.args.is_none() {
                return String::from("Error: no arguments given, but arguments are required!");
            } else {
                let min_args = self.req_args.len() + self.user_args.len();
                let max_args = min_args + self.optional_args.len();
                let args_provided = self.args.clone().unwrap().len();
                if args_provided > max_args || args_provided < min_args {
                    return String::from("Error: the wrong number of args were supplied!");
                }
            }
        }
        return (self.func)(self.args.clone());
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
}

pub fn build_tools() -> Vec<ToolCommand> {
    let mut tool_commands = Vec::new();
    let mut listproject = ToolCommand::new(
        "list_projects".to_string(),
        "coming soon".to_string(),
        list_projects,
    );
    listproject.req_args = vec![String::from("projects")];
    tool_commands.push(listproject);
    let mut createproject = ToolCommand::new(
        "create_project".to_string(),
        "creates a new project and saves it to the projects file to be reloaded on the next loop
        usage:
        create_project project_name"
            .to_string(),
        new_project,
    );
    createproject.req_args = vec![
        String::from("config"),
        String::from("upcoming_files"),
        String::from("upcoming_notes"),
        String::from("template"),
    ];
    createproject.user_args = vec![String::from("name")];
    tool_commands.push(createproject);
    let mut promoteproject = ToolCommand::new(
        "promote_project".to_string(),
        "promotes a project from upcoming to current, and sets up the distrobox.
Optional arguments: home=/path/to/distrobox/home name=project_name_to_promote"
            .to_string(),
        promote_project,
    );
    promoteproject.req_args = vec![
        String::from("projects"),
        String::from("tools"),
        String::from("files"),
        String::from("notes"),
        String::from("template"),
    ];
    promoteproject.optional_args = vec![String::from("home"), String::from("name")];
    tool_commands.push(promoteproject);
    let mut removeproject = ToolCommand::new(
        "remove_project".to_string(),
        "removes a project from the tool, deletes the files and notes, and deletes the distrobox."
            .to_string(),
        remove_project,
    );
    removeproject.optional_args = vec!["project".to_string()];
    removeproject.req_args = vec!["projects".to_string()];
    tool_commands.push(removeproject);
    return tool_commands;
}

pub fn list_projects(args: Option<Vec<ToolArgument>>) -> String {
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
    return table.get_table();
}

pub fn new_project(args: Option<Vec<ToolArgument>>) -> String {
    let given_args = args.unwrap();
    let mut config_path = PathBuf::new();
    let mut name = String::new();
    let mut files_path = PathBuf::new();
    let mut notes_path = PathBuf::new();
    let mut template_box = String::new();
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
        return String::from(
            "usage: newproject projectname\nprevious command was missing project name!",
        );
    }
    println!("gatered the data! name: {}", name);
    print_success("arguments parsed correctly!");
    println!("setting up files...");
    let mut project_path = config_path.clone();
    project_path.pop();
    project_path.push("projects");
    project_path.push(format!("{}.conf", &name));
    let file_create_res = File::create(&project_path);
    if file_create_res.is_err() {
        return format!(
            "Error failure to create project config file!\n{}\n{}",
            file_create_res.err().unwrap(),
            &project_path.display().to_string()
        );
    }
    files_path.push(&name);
    notes_path.push(&name);
    let files_dir_res = create_dir_all(&files_path);
    let notes_dir_res = create_dir_all(&notes_path);
    if files_dir_res.is_err() {
        return (format!(
            "Error failure to create project files folder!\n{}",
            files_dir_res.err().unwrap()
        ));
    }
    if notes_dir_res.is_err() {
        return (format!(
            "Error failure to create project files folder!\n{}",
            files_dir_res.err().unwrap()
        ));
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
    return new_project.generate_default_notes(&config_path);
}

pub fn promote_project(args: Option<Vec<ToolArgument>>) -> String {
    let mut project = String::new();
    let mut projects = Vec::new();
    let mut files = PathBuf::new();
    let mut notes = PathBuf::new();
    let mut tools = PathBuf::new();
    let mut template = String::new();
    let mut home = None;
    let mut given_args = Vec::new();
    if args.is_some() {
        given_args = args.unwrap();
    }
    for arg in given_args {
        match arg.name.as_str() {
            "name" => project = arg.string.clone().unwrap(),
            "projects" => projects = arg.projects.clone().unwrap(),
            "files" => files = arg.path.unwrap(),
            "notes" => notes = arg.path.unwrap(),
            "template" => template = arg.string.unwrap(),
            "home" => home = arg.path,
            "tools" => tools = arg.path.unwrap(),
            _ => {}
        }
    }
    let project_set = project.len() > 1;
    if !project_set {
        println!("{} : {}", project, project.len());
        println!("{}", project.len() > 1);
        let mut current_index = 0;
        for existing_project in &projects {
            println!("{}.) {}", current_index, existing_project.name);
            current_index += 1;
        }
        let response: usize = get_user_input("which project would you like to promote?")
            .parse()
            .unwrap();
        if response >= projects.len() {
            return format!("error invalid project selection!");
        }
        project = projects[response].name.clone();
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
            if !promote_res.to_lowercase().contains("success") {
                return format!("Error promoting project!\n{}", promote_res);
            }
        }
    }
    return String::from("Success!");
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
