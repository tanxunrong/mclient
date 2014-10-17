extern crate mclient;
use mclient::{Client,Mclient};

fn main() {
    let c = mclient::new("127.0.0.1",11211);
    match c {
        Ok(mut mc) => {
            let ret = mc.get("key");
            match ret {
                Ok(val) => { println!("val : {}",val); }
                Err(e) => { fail!("{}",e); }
            }
        }
        Err(e) => {  fail!("{}",e); }
    }
}
