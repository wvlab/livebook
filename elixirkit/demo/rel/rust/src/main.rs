use elixirkit_rust as elixirkit;

fn main() {
    let mut api = elixirkit::API::new();
    api.start("demo".to_string(), Box::new(|| {}), None);
    api.publish("log".to_string(), "Hello from Rust!".to_string());
    api.subscribe(Box::new(|name, data| match name.as_str() {
        "log" => println!("[client] {data}"),

        _ => panic!("unknown event {name}"),
    }));

    // TODO: wait for exit
    loop {
        std::thread::park();
    }
}
