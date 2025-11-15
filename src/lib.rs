use colored::Colorize;
use std::{
    fs::{File, OpenOptions, copy, create_dir_all, remove_dir_all, remove_file},
    io::Write,
    path::PathBuf,
    process::Command,
};
use walkdir::WalkDir;

#[derive(Default, Clone)]
pub struct Table {
    pub columns: Vec<usize>,
    pub headers: String,
    pub data: Vec<String>,
}

impl Table {
    pub fn build(&mut self, data: Vec<String>) -> Table {
        self.headers = data[0].clone();
        self.data = data[1..].to_vec();
        let header_vec: Vec<&str> = self.headers.split("|").collect();
        for header in header_vec {
            self.columns.push(header.len());
        }
        for data in &self.data {
            let data_vec: Vec<&str> = data.split("|").collect();
            for id in 0..self.columns.len() {
                if data_vec[id].len() > self.columns[id] {
                    self.columns[id] = data_vec[id].len();
                }
            }
        }
        for id in 0..self.columns.len() {
            if self.columns[id] % 2 != 0 {
                self.columns[id] += 1;
            }
        }
        return self.clone();
    }
    pub fn get_table(&self) -> String {
        let mut output = String::new();
        let mut spacer = String::new();
        let header_vec: Vec<&str> = self.headers.split("|").collect();
        for id in 0..self.columns.len() {
            spacer.push('|');
            let mut cell = String::new();
            let dashes = "-".repeat(self.columns[id]);
            spacer.push_str(&dashes);
            if header_vec[id].len() < self.columns[id] {
                let mut padding_needed = self.columns[id] - header_vec[id].len();
                if padding_needed % 2 != 0 {
                    padding_needed += 1;
                }
                let padding = padding_needed / 2;
                cell = format!(
                    "|{}{}{}",
                    " ".repeat(padding),
                    header_vec[id],
                    " ".repeat(padding)
                );
                while cell.len() != self.columns[id] {
                    if cell.len() > self.columns[id] + 1 {
                        cell.pop();
                    } else if cell.len() > self.columns[id] + 1 {
                        cell.push(' ');
                    } else {
                        break;
                    }
                }
                output.push_str(&cell);
            } else {
                cell = format!("|{}", header_vec[id]);
                output.push_str(&cell);
            }
        }
        output.push_str("|\n");
        spacer.push_str("|\n");
        output.push_str(&spacer);
        for data_line in self.data.clone() {
            let line_vec: Vec<&str> = data_line.split("|").collect();
            for id in 0..self.columns.len() {
                let mut cell = String::new();
                if line_vec[id].len() < self.columns[id] {
                    let mut padding_needed = self.columns[id] - line_vec[id].len();
                    if padding_needed % 2 != 0 {
                        padding_needed += 1;
                    }
                    let padding = padding_needed / 2;
                    cell = format!(
                        "|{}{}{}",
                        " ".repeat(padding),
                        line_vec[id],
                        " ".repeat(padding)
                    );
                    while cell.len() != self.columns[id] + 1 {
                        if cell.len() > self.columns[id] + 1 {
                            cell.pop();
                        } else if cell.len() < self.columns[id] + 1 {
                            cell.push(' ');
                        } else {
                            break;
                        }
                    }
                } else {
                    cell = format!("|{}", line_vec[id]);
                }
                output.push_str(&cell);
            }
            output.push_str("|\n");
            output.push_str(&spacer);
        }
        return output;
    }
}

pub struct Server {
    pub address: String,
    pub id: usize,
}

#[derive(Clone)]
pub struct Message {
    pub source: Destination,
    pub destination: Destination,
    pub content: String,
}

#[derive(Clone, PartialEq)]
pub enum Destination {
    Console,
    Server,
    Control,
}

#[derive(Default, Clone)]
pub struct Project {
    pub name: String,
    pub files: PathBuf,
    pub notes: PathBuf,
    pub current: bool,
    pub boxname: String,
    pub config: PathBuf,
}

impl Project {
    pub fn generate_default_notes(&self, config_folder: &PathBuf) -> String {
        let mut notes_template = config_folder.clone();
        notes_template.pop();
        notes_template.push("note_templates");
        if self.name.contains("external") {
            notes_template.push("external");
        } else if self.name.contains("internal") {
            notes_template.push("internal");
        } else if self.name.contains("vishing") {
            notes_template.push("vishing");
        } else if self.name.contains("phishing") {
            notes_template.push("phishing");
        } else if self.name.contains("webapp") {
            notes_template.push("webapp");
        }
        let walkdir = WalkDir::new(&notes_template);
        for res in walkdir {
            if res.is_ok() {
                let entry = res.unwrap();
                let file_name = entry.file_name().to_string_lossy().to_string();
                if file_name.contains(".md") {
                    let mut temp_path = self.notes.clone();
                    temp_path.push(&file_name);
                    let copy_res = copy(entry.path(), &temp_path);
                    if copy_res.is_err() {
                        return (format!(
                            "Error copying note file {} to {}",
                            file_name,
                            temp_path.display()
                        ));
                    }
                    copy_res.unwrap();
                }
            }
        }
        return String::from("Success!");
    }

    pub fn promote_project(
        &mut self,
        files: &PathBuf,
        notes: &PathBuf,
        template: String,
        tools: &PathBuf,
        home: Option<PathBuf>,
    ) -> String {
        let mut new_files_path = files.clone();
        let mut new_notes_path = notes.clone();
        new_files_path.push(&self.name);
        new_notes_path.push(&self.name);
        let files_create_res = create_dir_all(&new_files_path);
        if files_create_res.is_err() {
            return (format!(
                "Error creating new files folder!\n{}",
                files_create_res.err().unwrap()
            ));
        }
        let notes_create_res = create_dir_all(&new_notes_path);
        if notes_create_res.is_err() {
            return (format!(
                "Error creating new notes folder!\n{}",
                notes_create_res.err().unwrap()
            ));
        }
        files_create_res.unwrap();
        notes_create_res.unwrap();
        let file_walkdir = WalkDir::new(&self.files);
        let notes_walkdir = WalkDir::new(&self.notes);
        for res in file_walkdir {
            if res.is_ok() {
                let entry = res.unwrap();
                if entry.file_type().is_file() {
                    let mut temp_path = new_files_path.clone();
                    temp_path.push(entry.file_name());
                    let copy_res = copy(entry.path(), temp_path);
                    if copy_res.is_ok() {
                        copy_res.unwrap();
                    }
                }
            }
        }
        for res in notes_walkdir {
            if res.is_ok() {
                let entry = res.unwrap();
                if entry.file_type().is_file() {
                    let mut temp_path = new_notes_path.clone();
                    temp_path.push(entry.file_name());
                    let copy_res = copy(entry.path(), temp_path);
                    if copy_res.is_ok() {
                        copy_res.unwrap();
                    }
                }
            }
        }
        let old_file_remove_res = remove_dir_all(&self.files);
        if old_file_remove_res.is_err() {
            println!(
                "{}",
                "Error removing upcoming files directory, manual clean up required!".red()
            );
        } else {
            old_file_remove_res.unwrap();
        }
        let old_note_remove_res = remove_dir_all(&self.notes);
        if old_note_remove_res.is_err() {
            println!(
                "{}",
                "Error removing upcoming notes directory, manual cleanup required!".red(),
            );
        } else {
            old_note_remove_res.unwrap();
        }

        self.current = true;
        self.files = new_files_path;
        self.notes = new_notes_path;
        self.save_project();
        let distrobox_res = self.create_distrobox(template, tools, home);
        if !distrobox_res.to_lowercase().contains("Success") {
            return format!(
                "Error creating distrobox!\n{}\n\nThe project was still promoted, but the distrobox has not been created!",
                distrobox_res
            );
        }
        return String::from("Success!");
    }

    pub fn save_project(&self) -> String {
        if self.config.exists() {
            let remove_res = remove_file(&self.config);
            if remove_res.is_err() {
                return format!("Error removing old config!\n{}", remove_res.err().unwrap());
            }
            remove_res.unwrap();
        }
        let conf_open_create_res = File::create(&self.config);
        if conf_open_create_res.is_err() {
            return format!(
                "Error creating new config file!\n{}",
                conf_open_create_res.err().unwrap()
            );
        }
        let mut conf_file = conf_open_create_res.unwrap();
        let mut config_string = format!(
            "name|{}\nfiles|{}\nnotes|{}\nboxname|{}\nconfig|{}\n",
            self.name,
            self.files.display(),
            self.notes.display(),
            self.boxname,
            self.config.display(),
        );
        if self.current {
            config_string.push_str("stage|current");
        } else {
            config_string.push_str("stage|upcoming");
        }
        write!(conf_file, "{}", config_string).unwrap();
        return String::from("Success!");
    }

    pub fn remove_project(&self) -> String {
        let files_remove_res = remove_dir_all(&self.files);
        if files_remove_res.is_err() {
            return format!(
                "Error removing files directory!\n{}",
                files_remove_res.err().unwrap()
            );
        }
        let notes_remove_res = remove_dir_all(&self.notes);
        if notes_remove_res.is_err() {
            return format!(
                "Error removing notes directory!\n{}",
                notes_remove_res.err().unwrap()
            );
        }
        let config_remove_res = remove_file(&self.config);
        if config_remove_res.is_err() {
            return format!(
                "Error removing config file!\n{}",
                config_remove_res.err().unwrap()
            );
        }
        let db_remove_res = Command::new("distrobox")
            .arg("rm")
            .arg("--root")
            .arg(&self.boxname)
            .status();
        if db_remove_res.is_err() {
            return format!(
                "Error deleting distrobox!\n{}",
                db_remove_res.err().unwrap()
            );
        }
        db_remove_res.unwrap();
        return String::from("Success!");
    }

    pub fn create_distrobox(
        &self,
        template: String,
        tools: &PathBuf,
        home: Option<PathBuf>,
    ) -> String {
        println!("stopping project distrobox and template distrobox.");
        println!("ignore any errors about the distrobox not existing.");
        let pdb_stop_res = Command::new("distrobox")
            .arg("stop")
            .arg("--root")
            .arg(&self.boxname)
            .status();
        if pdb_stop_res.is_err() {
            return format!(
                "Error stopping project distrobox!\n{}",
                pdb_stop_res.err().unwrap()
            );
        }
        let tdb_stop_res = Command::new("distrobox")
            .arg("stop")
            .arg("--root")
            .arg(&template)
            .status();
        if tdb_stop_res.is_err() {
            return format!(
                "Error stopping template box!\n{}",
                tdb_stop_res.err().unwrap()
            );
        }
        pdb_stop_res.unwrap();
        tdb_stop_res.unwrap();
        let db_remove_res = Command::new("distrobox")
            .arg("rm")
            .arg("--root")
            .arg(&self.boxname)
            .status();
        if db_remove_res.is_err() {
            return format!(
                "Error removing distrobox!\n{}",
                db_remove_res.err().unwrap()
            );
        }
        db_remove_res.unwrap();
        let db_create_res = Command::new("distrobox")
            .arg("create")
            .arg("--root")
            .arg("--clone")
            .arg(template)
            .arg("--init")
            .arg("--volume")
            .arg(format!("{}:/pentest:rw", &self.files.display()))
            .arg(format!("{}:/tools:rw", tools.display()))
            .arg("--name")
            .arg(&self.boxname)
            .status();
        if db_create_res.is_err() {
            return format!(
                "Error creating distrobox!\n{}",
                db_create_res.err().unwrap()
            );
        }
        db_create_res.unwrap();
        println!("{}", "distrobox created!".green());
        println!("starting it up to do some setup stuff.");
        let start_res = Command::new("distrobox")
            .arg("enter")
            .arg("--root")
            .arg(&self.boxname)
            .arg("--")
            .arg("exit")
            .status();
        if start_res.is_err() {
            return format!("Error starting distrobox!\n{}", start_res.err().unwrap());
        }
        return String::from("Success!");
    }
}
