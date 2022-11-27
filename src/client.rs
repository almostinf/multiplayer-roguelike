use std::net::TcpStream;
use url::Url;

use tungstenite::{connect, WebSocket, http::Response, stream::MaybeTlsStream, Message};


pub struct Client {
    pub socket : WebSocket<MaybeTlsStream<TcpStream>>,
    _response : Response<()>,
}

impl Client {
    pub fn new(request : Url) -> Self {
        let (socket, response) = connect(request).expect("Can't connect");
        Client {
            socket,
            _response : response,
        }
    }

    pub fn get_message(&mut self) -> Result<(String, String), String> {
        if self.socket.can_read() {
            let message = self.socket.read_message().unwrap().to_string();
            let mut key = String::new();
            let mut turn = false;
            let mut skip = true;
            let mut value = String::new();
            for (i, ch) in message.chars().enumerate() {
                if i == 0 || i == 1 || i == message.len() - 1 {
                    continue;
                } 
                if skip {
                    skip = false;
                    continue;
                }
                if ch == '"' && !turn {
                    turn = true;
                    skip = true;
                } else if !turn {
                    key.push(ch);
                } else if ch == '"' && turn {
                    continue;
                } else if turn {
                    value.push(ch);
                }
            }
            Ok((key, value))
        } else {
            Err("No messages in stream".to_string())
        }
    }

    pub fn send_message(&mut self, message : Vec<u8>) {
        self.socket.write_message(Message::Binary(message)).expect("Can't send message");
    }
}
