#![feature(globs)]

use std::io::*;
use std::error::*;
use std::io::net::ip::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use std::cell::RefCell;

pub struct Client {
    addr : SocketAddr,
    conn : RefCell<TcpStream>
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

#[deriving(Show)]
pub enum Response {
    Stored,
    NotStored,
    InvalidCmd,
    ClientErr(String),
    ServerErr(String)
}

impl Client {

    pub fn new(addr:&str) -> McResult<Client> {
        let ad : Option<SocketAddr> = FromStr::from_str(addr);
        match ad {
            Some(addr) => {
                let conn = try!(TcpStream::connect_timeout(addr,Duration::seconds(1)));
                Ok(Client{addr:addr,conn:RefCell::new(conn)})
            },
            None => {
                Err(FromError::from_error(ClientError{desc:"invalid addr".into_string()}))
            }
        }
    }

    pub fn set(&self,key:&str,flag:u16,expire:uint,data:&str) -> McResult<Response> {
        let cmd = format_args!(std::fmt::format,"set {} {} {} {}\r\n{}\r\n",key,flag,expire,data.as_slice().as_bytes().len(),data);
        println!("{}",cmd);

        let mut conn = self.conn.borrow_mut();
        conn.set_timeout(Some(5000u64));

        try!(conn.write(cmd.as_slice().as_bytes()));

        let ret = try!(conn.read_to_string());

        if ret.starts_with("STORED\r\n") { return Ok(Response::Stored); }
        else if ret.starts_with("NOT_STORED\r\n") { return Ok(Response::NotStored); }
        else if ret.starts_with("ERROR\r\n" ) { return Ok(Response::InvalidCmd); }
        else if ret.starts_with("CLIENT_ERROR") { let err = ret.slice(12,ret.len()-4).into_string();
            return Ok(Response::ClientErr(err));}
        else if ret.starts_with("SERVER_ERROR") {  let err = ret.slice(12,ret.len()-4).into_string();
            return Ok(Response::ServerErr(err));}
        else { return Ok(Response::InvalidCmd); }
    }

}

