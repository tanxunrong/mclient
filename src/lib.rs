
#![crate_name="mclient"]

#![feature(globs)]
use std::io::*;
use std::io::net::ip::SocketAddr;
use std::from_str::FromStr;
use std::time::Duration;

pub trait Mclient {
    fn get(&mut self,key:&str) -> Result<String,Error> ;
}

pub struct Client {
    addr : SocketAddr,
    conn : Option<TcpStream>
}

#[deriving(PartialEq, Eq, Clone, Show)]
pub enum ErrorKind {
    InvalidHost,
    InterIoErr(IoError)
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
            Ok(Client{addr:SocketAddr{ip:ip,port:port},conn:None})
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

impl Mclient for Client {
    fn get(&mut self,key:&str) -> Result<String,Error> {
        assert!(!key.contains(" "));
        assert!(!key.contains_char('\t'));
        assert!(!key.contains_char('\n'));
        let cmd = String::from_str("get ") + key.into_string() + "\r\n".into_string();
        if self.conn.is_none() {
            match TcpStream::connect_timeout(self.addr,Duration::seconds(1)) {
                Ok(c) => {
                    self.conn = Some(c);
                },
                Err(e) => {
                    return Err(Error{
                        desc : "fail to conn",
                        detail : None,
                        kind : InterIoErr(e)
                    })
                }
            }
        }

        let mut tc = self.conn.take().unwrap();
        tc.write_str(cmd.as_slice());
        let mut ret = [0u8,..100];
        match tc.read(ret) {
            Ok(nread) => {
                println!("{} read",nread);
                let v : Vec<char> = ret.iter().map(|x| x.to_ascii().to_char()).collect();
                Ok(String::from_chars(v.as_slice()))
            },
            Err(err) => {
                    Err(Error{
                        desc : "fail to read",
                        detail : None,
                        kind : InterIoErr(err)
                    })
            }
        }
    }
}

#[test]
fn test_get() {
    let mut c = new("127.0.0.1",11211);
    assert!(c.is_ok());
    match c {
        Ok(mut mc) => { let ret = mc.get("foo");println!("ret {}",ret); }
        Err(e) => { fail!("not ok"); }
    }
}

