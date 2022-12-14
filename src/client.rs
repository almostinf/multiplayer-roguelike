use std::net::TcpStream;
use url::Url;

use websocket::{ClientBuilder, Message, sync::Client};


/// Used to send / receive data from the server
pub struct ClientHandler {
    socket : Client<TcpStream>,
    pub messages : Vec<(String, String)>,
}


impl ClientHandler {

    /// Create new ClientHandler and set timeout for reading messages in stream
    pub fn new(request : Url) -> Self {
        let client = ClientBuilder::new(request.as_str())
		.unwrap()
		// .add_protocol("rust-websocket")
		.connect_insecure()
		.unwrap();

        // set timeout
        client.stream_ref().set_read_timeout(Some(std::time::Duration::from_millis(15))).unwrap();

        ClientHandler {
            socket : client,
            messages : Vec::<(String, String)>::new(),
        }
    }

    /// Сollects all messages from the stream
    pub fn get_messages(&mut self) {

        for msg in self.socket.incoming_messages() {
            match msg {
                Ok(m) => {
                    match m {
                        websocket::OwnedMessage::Text(data) => {

                            // deserialize json
                            let mut key = String::new();
                            let mut turn = false;
                            let mut skip = false;
                            let mut skip_x2 = false;
                            let mut count = 0;
                            let mut value = String::new();

                            for (i, ch) in data.chars().enumerate() {
                                if i == 0 || i == 1 || i == data.len() - 1 || i == data.len() - 2 {
                                    continue;   
                                }
                                if skip {
                                    skip = false;
                                    continue;
                                }
                                if skip_x2 {
                                    if count == 1 {
                                        skip_x2 = false;
                                    }
                                    count += 1;
                                    continue;
                                }
                                if ch == '"' && !turn {
                                    turn = true;
                                    skip_x2 = true;
                                } else if !turn {
                                    key.push(ch);
                                } else if ch == '\\' && turn {
                                    continue;
                                } else if turn {
                                    value.push(ch);
                                }
                            }
                            println!("message: {} {}", key, value);
                            self.messages.push((key, value));
                        }
                        _ => (),
                    }
                }
                Err(_) => break,
            }
        }
    }

    /// Send messages to the server
    pub fn send_message(&mut self, msg : Vec<u8>) {
        self.socket.send_message(&Message::binary(msg)).expect("Can't send message");
    }

}
