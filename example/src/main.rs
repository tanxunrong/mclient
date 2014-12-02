extern crate mclient;
use mclient::{Client};

fn main() {
    let c = Client::new("127.0.0.1:11211").unwrap();
    match c.set("go",0u16,0u,"for") {
        Ok(ret) => { println!("ret {}",ret); }
        Err(e) => { panic!(e); }
    }
}
