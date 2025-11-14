use std::{
    fs::{copy, create_dir_all},
    path::PathBuf,
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
}
