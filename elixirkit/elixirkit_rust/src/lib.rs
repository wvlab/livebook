use std::{
    io::{Read as _, Write as _},
    sync::{Arc, Mutex},
};

pub type ReadyHandler = Box<dyn FnOnce() + Send + 'static>;
pub type ExitHandler = Box<dyn Fn(i32) + Send + 'static>;
pub type EventHandler = Box<dyn Fn(String, String) + Send + Sync>;

pub struct API {
    release: Option<Release>,
}

impl API {
    pub fn new() -> Self {
        API { release: None }
    }

    pub fn start(&mut self, name: String, ready: ReadyHandler, exited: Option<ExitHandler>) {
        self.release = Some(Release::new(name, ready, exited));
    }

    pub fn publish(&self, name: String, data: String) {
        if let Some(release) = &self.release {
            release.send(format!("{}:{}", name, data));
        }
    }

    pub fn subscribe(&self, handler: EventHandler) {
        if let Some(release) = &self.release {
            release.subscribe(handler);
        }
    }
}

struct Release {
    process: std::process::Child,
    stream: Arc<Mutex<std::net::TcpStream>>,
    handlers: Arc<Mutex<Vec<EventHandler>>>,
}

impl Release {
    fn new(name: String, ready: ReadyHandler, _exited: Option<ExitHandler>) -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let process = std::process::Command::new(format!("./rel/bin/{}", name))
            .arg("start")
            .env("ELIXIRKIT_PORT", port.to_string())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit())
            .spawn()
            .unwrap();

        println!("Started process with port {}", port);

        let (stream, addr) = listener.accept().unwrap();
        println!("Connected from {}", addr);
        let stream = Arc::new(Mutex::new(stream));
        let handlers: Arc<Mutex<Vec<EventHandler>>> = Arc::new(Mutex::new(Vec::new()));
        let handlers_clone = Arc::clone(&handlers);

        std::thread::spawn(move || {
            ready();
        });

        let stream_clone = stream.clone();
        std::thread::spawn(move || {
            let mut stream = stream_clone.lock().unwrap().try_clone().unwrap();

            loop {
                let mut length_buf = [0u8; 4];
                match stream.read_exact(&mut length_buf) {
                    Ok(()) => {
                        let length = u32::from_be_bytes(length_buf) as usize;

                        let mut message_buf = vec![0u8; length];
                        match stream.read_exact(&mut message_buf) {
                            Ok(()) => {
                                let received = String::from_utf8_lossy(&message_buf);
                                let (name, data) = received.split_once(":").unwrap();
                                let handlers = handlers_clone.lock().unwrap();
                                for handler in handlers.iter() {
                                    handler(name.to_string(), data.to_string());
                                }
                            }
                            Err(e) => {
                                println!("Error reading message: {}", e);
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error reading length: {}", e);
                        break;
                    }
                }
            }
        });

        Release {
            process,
            stream,
            handlers,
        }
    }

    fn send(&self, message: String) {
        println!("Sending: {:?}", message);
        let mut stream = self.stream.lock().unwrap();
        let payload = message.as_bytes();
        let length = payload.len() as u32;
        stream.write_all(&length.to_be_bytes()).unwrap();
        stream.write_all(payload).unwrap();
        stream.flush().unwrap();
    }

    fn subscribe(&self, handler: EventHandler) {
        self.handlers.lock().unwrap().push(handler);
    }
}
