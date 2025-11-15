use std::collections::HashMap;
use std::fs::{File, create_dir_all, read_to_string, remove_file};
use std::io::Write;
use std::path::PathBuf;

use crate::get_user_input;
use crate::print_success;
use crate::{crytpo, print_error};

pub fn install(config: &PathBuf) -> bool {
    let mut new = true;
    let mut config_folder = config.clone();
    config_folder.pop();
    let mut config_file = config_folder.clone();
    config_file.push("config.conf");
    let mut project_config = config_folder.clone();
    let mut note_templates_path = config_folder.clone();
    project_config.push("projects");
    note_templates_path.push("note_templates");
    let mut settings = HashMap::new();
    if !config_folder.exists() {
        let dir_create_res = create_dir_all(&config_folder);
        if dir_create_res.is_err() {
            print_error(
                "error creating configuration directory!",
                Some(dir_create_res.err().unwrap().to_string()),
            );
            return false;
        }
        print_success("configuration folder created successfully!");
    } else {
        print_success("configuration folder already exists!");
    }
    if config_file.exists() {
        if get_user_input("the config file already exists, would you like to delete it and create a full new one?").to_lowercase().contains("y"){
            let remove_res = remove_file(&config_file);
            if remove_res.is_err(){
                print_error("error removing file, please manually delete and try again...", Some(remove_res.err().unwrap().to_string()));
                return false;
            }
            if project_config.exists(){
                let remove_res = remove_file(&project_config);
                if remove_res.is_err(){
                    print_error("error removing projects file", Some(remove_res.err().unwrap().to_string()));
                    return false;
                }
            }
        }
        else{
            new = false;
            let config_read_res = read_to_string(&config_file);
            if config_read_res.is_err(){
                print_error("error reading config file!", Some(config_read_res.err().unwrap().to_string()));
                return false;
            }
            let config_string = config_read_res.unwrap();
            for line in config_string.lines(){
                if line.contains(":"){
                    let line_vec: Vec<&str> = line.split(":").collect();
                    if settings.contains_key(line_vec[0]){
                        settings.remove(line_vec[0]);
                        settings.insert(line_vec[0].to_string(), line_vec[1].to_string());
                    }
                    else{
                        settings.insert(line_vec[0].to_string(), line_vec[1].to_string());
                    }
                }
            }
            print_success("read existing config file successfully!");
            println!("entering edit loop...");
            loop{
                println!("{}", config_string);
                let setting = get_user_input("which setting would you like to change? (ENTER DONE IN ALL CAPS WHEN YOU'RE FINISHED");
                let value = get_user_input("what would you like to change it to?");
                if setting.contains("DONE"){
                    break;
                }
                else {
                    if settings.contains_key(&setting){
                        settings.remove(&setting);
                        settings.insert(setting, value);
                    }
                    else{
                        settings.insert(setting, value);
                    }
                }
            }
        }
    }
    if !project_config.exists() {
        let projects_create_res = create_dir_all(&project_config);
        if projects_create_res.is_err() {
            print_error(
                "error creating projects directory!",
                Some(projects_create_res.err().unwrap().to_string()),
            );
        }
    }
    if !note_templates_path.exists() {
        let note_template_create_res = create_dir_all(&note_templates_path);
        if note_template_create_res.is_err() {
            print_error(
                "error createing note_templates directory!",
                Some(note_template_create_res.err().unwrap().to_string()),
            );
        }
    }
    if new {
        println!("server_address|127.0.0.1:31337");
        println!("key_file|{}/key", &config_folder.display());
        println!("distrobox|yes");
        if !get_user_input("are these defaults ok?")
            .to_lowercase()
            .contains("y")
        {
            let server_address = get_user_input("what is your server address then?");
            let key_file = get_user_input("what is your key file then?");
            if get_user_input("will you be using distrobox for your attack environments?")
                .to_lowercase()
                .contains("y")
            {
                settings.insert("distrobox".to_string(), "yes".to_string());
                settings.insert(
                    "templatebox".to_string(),
                    get_user_input("name of your tempalte distrobox?"),
                );
            } else {
                settings.insert("distrobox".to_string(), "no".to_string());
            }
            settings.insert("server_address".to_string(), server_address);
            settings.insert("key_file".to_string(), key_file);
        } else {
            settings.insert("server_address".to_string(), "127.0.0.1:31337".to_string());
            settings.insert(
                "key_file".to_string(),
                format!("{}/key", config_folder.display()),
            );
            settings.insert("distrobox".to_string(), "yes".to_string());
            settings.insert(
                "templatebox".to_string(),
                get_user_input("name of the distrobox you will use?"),
            );
        }
        settings.insert("current_files".to_string(), get_user_input("full path to where you want your current project's files stored? example: /home/pyro/projects/current"));
        settings.insert("current_notes".to_string(), get_user_input("full path to where you want your current project's notes stored example: /home/pyro/notes/current"));
        settings.insert("upcoming_files".to_string(),get_user_input("full path to where you want your upcoming project's files stored example: /home/pyro/projects/upcoming"));
        settings.insert("upcoming_notes".to_string(), get_user_input("full path to where you want your upcoming project's notes stored exmple: /home/pyro/notes/upcoming"));
        settings.insert(
            "tools".to_string(),
            get_user_input(
                "full path to where you store your custom tools (like those from github)?",
            ),
        );
        settings.insert("terminal".to_string(), get_user_input("command used to launch your terminal while executing a command (for exmaple konsole in kde is konsole -e)?"));
        print_success("sweet, we have all we need, writing config file...");
        let out_file_res = File::create_new(&config_file);
        if out_file_res.is_err() {
            print_error(
                "error creating config file!",
                Some(out_file_res.err().unwrap().to_string()),
            );
            return false;
        }
        let mut out_file = out_file_res.unwrap();
        for setting in settings.keys() {
            let outline = format!("{}|{}\n", setting, settings[setting]);
            let write_res = out_file.write(outline.as_bytes());
            if write_res.is_err() {
                print_error(
                    "error writing to config file",
                    Some(write_res.err().unwrap().to_string()),
                );
                return false;
            } else {
                write_res.unwrap();
            }
        }
        print_success("excellent we have created the client's config file!");
        println!("creating projects config file and adding the default project...");
        project_config.push("default.conf");
        let projects_file_res = File::create_new(project_config);
        if projects_file_res.is_err() {
            print_error(
                "error creating project config file!",
                Some(projects_file_res.err().unwrap().to_string()),
            );
            return false;
        }
        for key in settings.keys() {
            println!("{} : {}", key, settings[key]);
        }
        let mut project_file = projects_file_res.unwrap();
        let mut out_line = format!(
            "name|default\nstage|current\nfiles|{}\nnotes|{}",
            settings["current_files"], settings["current_notes"]
        );
        if settings["distrobox"] == "yes".to_string() {
            out_line = format!("{}\nboxname|{}", out_line, settings["templatebox"]);
        }
        let write_res = project_file.write(out_line.as_bytes());
        if write_res.is_err() {
            print_error(
                "error writing to projects config file!",
                Some(write_res.err().unwrap().to_string()),
            );
            return false;
        }
    }
    println!("generating a new key for encryption...");
    let key = crytpo::generate_key();
    let mut key_path = config_folder.clone();
    key_path.push("key");
    if key_path.exists() {
        let remove_res = remove_file(&key_path);
        if remove_res.is_err() {
            print_error(
                "error removing keyfile",
                Some(remove_res.err().unwrap().to_string()),
            );
            return false;
        }
    }
    let key_file_res = File::create_new(key_path);
    if key_file_res.is_err() {
        print_error(
            "error making key file!",
            Some(key_file_res.err().unwrap().to_string()),
        );
        return false;
    }
    let mut key_file = key_file_res.unwrap();
    key_file.write(&key).unwrap();
    print_success("client successfully installed!");
    print_success("please re-run this tool to use it!");
    return true;
}
