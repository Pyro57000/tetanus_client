use tokio;
use std::{io::Write, net::TcpStream};

use crate::print_error;


pub async fn send_to_server(input: String, address: String) -> Option<String>{
    let connect_res = TcpStream::connect(address);
    if connect_res.is_err(){
        print_error("error connection to server", Some(connect_res.err().unwrap().to_string()));
        return Some(String::from("failed to connect to server!"));
    }
    let mut stream = connect_res.unwrap();
    let server_send_line = format!("1|||command|||0|||{}", input);
    stream.write(server_send_line.as_bytes()).unwrap();
    return None;
}