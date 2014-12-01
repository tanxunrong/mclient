#![feature(globs)]

use std::io::*;
use std::io::net::ip::SocketAddr;
use std::str::FromStr;
use std::error::*;
use std::time::Duration;
use std::default::Default;
use std::cell::RefCell;

pub struct Client {
    addr : SocketAddr,
    conn : RefCell<Option<TcpStream>>
}

#[deriving(Show,Clone)]
pub enum Failure {
    Io(IoError),
    Client(ClientError)
}

impl FromError<IoError> for Failure {
    fn from_error(err:IoError) -> Failure {
        Failure::Io(err)
    }
}

#[deriving(Show,Clone)]
pub struct ClientError {
    desc : String
}

impl Error for ClientError {
    fn description(&self) -> &str {
        self.desc.as_slice()
    }
}

impl FromError<ClientError> for Failure {
    fn from_error(err:ClientError) -> Failure {
        Failure::Client(err)
    }
}

pub type McResult<T> = Result<T,Failure>;

impl Client {

    pub fn new(addr:&str) -> McResult<Client> {
        let addr : Option<SocketAddr> = FromStr::from_str(addr);
        match addr {
            Some(ad) => {
                Ok(Client{addr:ad,conn:RefCell::new(None)})
            },
            None => {
                Err(Failure::Client(ClientError{desc:"invalid addr".into_string()}))
            }
        }
    }
/*
    fn get_conn(&self) -> McResult<Client> {
        let mut tc = self.conn.borrow_mut();
        if tc.is_none() {
            let t = try!(TcpStream::connect_timeout(self.addr,Duration::seconds(1)));
            *tc = Some(t);
        }
        Ok(*self)
    }
*/
}

#[test]
fn test_new_mc() {
    let mut c = Client::new("127.0.0.1:11211");
    assert!(c.is_ok());
}

