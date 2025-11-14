use crate::Destination;
use crate::Message;
use crate::Project;
use crate::lib::Table;
use crate::print_error;
use crate::print_success;
use crate::save_project;
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
            args: None,
            func,
        }
    }

    pub fn execute(&self) -> String {
        if self.req_args.len() > 0 {
            if self.args.is_none() {
                return String::from("Error: no arguments given, but arguments are required!");
            } else if self.args.clone().unwrap().len() != self.req_args.len() + self.user_args.len()
            {
                return String::from("Error: the wrong number of args were supplied!");
            }
        }
        return (self.func)(self.args.clone());
    }
}

#[derive(Clone, Default)]
pub struct ToolArgument {
    pub name: String,
    pub user_supplied: bool,
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
    println!("reached the new_project function");
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
    let conf_file = file_create_res.unwrap();
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
    save_project(&new_project, &project_path);
    return String::from("Success!");
}
