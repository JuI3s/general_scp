use std::{
    io::{BufRead, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::application::quorum::QuorumNode;

pub struct TCPServer {
    pub local_node: QuorumNode,
}

impl TCPServer {
    pub fn listen(&self) {
        match self.local_node.ip_addr {
            Some(ip) => {
                let listener = TcpListener::bind(ip).unwrap();
                for stream in listener.incoming() {
                    let stream = stream.unwrap();

                    handle_connection(stream);
                }
            }
            None => {
                panic!("No IP address provided for the local node.")
            }
        }
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let http_request: Vec<_> = buf_reader
        .lines()
        .map(|result| result.unwrap())
        .take_while(|line| !line.is_empty())
        .collect();

    println!("Request: {http_request:#?}");
}
