
#![crate_name="mclient"]

#![feature(globs)]
use std::io::*;
use std::io::net::ip::SocketAddr;
use std::from_str::FromStr;

pub struct Client {
    addr : SocketAddr
}

#[deriving(PartialEq, Eq, Clone, Show)]
pub enum ErrorKind {
    InvalidHost,
    InvalidPort
}

#[deriving(PartialEq, Eq, Clone, Show)]
pub struct Error {
    desc : &'static str,
    detail: Option<String>,
    kind : ErrorKind
}

pub fn new(host:&str,port:u16) -> Result<Client,Error> {
    let ipo : Option<IpAddr> = FromStr::from_str(host);
    match ipo {
        Some(ip) => {
            Ok(Client{addr:SocketAddr{ip:ip,port:port}})
        },
        None => {
            Err(Error{
                desc : "invalid host",
                kind : InvalidHost,
                detail : None
            })
        }
    }
}

#[test]
fn test_new_client() {
    let c = new("127.0.0.1",11211u16);
    assert!(c.is_ok());
}
