#![feature(globs)]

extern crate libc;

use std::io::*;
use std::error::*;
use std::io::net::ip::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use std::cell::RefCell;
use proto::protocol_binary_request_header as Reqhead;
use proto::protocol_binary_response_header as Reshead;
mod proto;

pub struct Client {
    addr : SocketAddr,
    conn : TcpStream
}

#[deriving(Show,Clone)]
pub enum Failure {
    Io(IoError),
    Client(String),
    Server(String)
}

impl FromError<IoError> for Failure {
    fn from_error(err:IoError) -> Failure {
        Failure::Io(err)
    }
}

impl Error for Failure {
    fn description(&self) -> &str {
        match *self {
            Failure::Io(ref e) => { e.description() },
            Failure::Client(ref s) => { s.as_slice() } // let err = format_args!(std::fmt::format,"Client Error : {}",s); err.as_slice() },
            Failure::Server(ref s) => { s.as_slice() } //let err = format_args!(std::fmt::format,"Server Error : {}",s); err.as_slice() }
        }
    }
}

pub type McResult<T> = Result<T,Failure>;

#[deriving(Show)]
pub struct Item {
    key:String,
    flag:u16,
    val:Vec<u8>
}

pub trait FromMcItem {
    fn from_item(item:&Item) -> McResult<Self>;
}

#[deriving(Show)]
pub enum Response {
    Stored,
    NotStored,
    InvalidCmd,
    Deleted,
    NotFound,
    Value(Item)
}

struct Parser<T> {
    reader : T
}

impl <'a,T:Reader> Parser<T> {

    fn new(reader: T) -> Parser<T> {
        Parser { reader: reader }
    }

    #[inline]
    fn expect_char(&mut self, refchar: char) -> McResult<()> {
        if try!(self.reader.read_byte()) as char == refchar {
            Ok(())
        } else {
            Err(Failure::Server("Invalid byte in response".into_string()))
        }
    }

    #[inline]
    fn expect_newline(&mut self) -> McResult<()> {
        match try!(self.reader.read_byte()) as char {
            '\n' => Ok(()),
            '\r' => self.expect_char('\n'),
            _ => Err(Failure::Server("Expect new line,Invalid byte in response".into_string()))
        }
    }

    fn read_line(&mut self,len:Option<uint>) -> McResult<Vec<u8>> {
        let mut rv = vec![];

        loop {
            let b = try!(self.reader.read_byte());
            match b as char {
                '\n' => { break; }
                '\r' => {
                    try!(self.expect_char('\n'));
                    break;
                },
                _ => { 
                    rv.push(b); 
                    match len {
                        Some(l) => { 
                            if rv.len() > l { return Err(Failure::Server("Expect no more".into_string())); }
                        },
                        None => {},
                    }
                }
            };
        }

        Ok(rv)
    }

    fn read_string_line(&mut self) -> McResult<String> {
        match String::from_utf8(try!(self.read_line(None))) {
            Err(_) => {
                Err(Failure::Server("Expect string,Invalid byte in response".into_string()))
            }
            Ok(value) => Ok(value)
        }
    }

    fn parse_value(&mut self) -> McResult<Response> {
        let ret = self.read_string_line().unwrap();
        let line = ret.as_slice();
        if line.len() < 5 {
            return Err(Failure::Server("Expect more,Invalid byte in response".into_string()));
        }
        if line.starts_with("STORED") {
            Ok(Response::Stored)
        } 
        else if line.starts_with("NOT_STORED") {
            Ok(Response::NotStored)
        }
        else if line.starts_with("ERROR") {
            Ok(Response::InvalidCmd)
        }
        else if line.starts_with("NOT_FOUND") {
            Ok(Response::NotFound)
        }
        else if line.starts_with("DELETED") {
            Ok(Response::Deleted)
        }
        else if line.starts_with("CLIENT_ERROR") {
            let err = line.slice(12,line.len()-4).into_string();
            Err(Failure::Client(err))
        }
        else if line.starts_with("SERVER_ERROR") {
            let err = line.slice(12,line.len()-4).into_string();
            Err(Failure::Server(err))
        }
        else if line.starts_with("VALUE") {
            let mut mess : Vec<String> = vec![];
            for s in line.split_str(" ") {
                mess.push(String::from_str(s))
            }
            if mess.len() != 4 {
                return Err(Failure::Server("invalid Value response".into_string()));
            }

            let datalen:Option<uint> = FromStr::from_str(mess[3].as_slice());
            let next = self.read_line(datalen).unwrap();

            let end = self.read_string_line().unwrap();
            if end.as_slice() != "END" {
                return Err(Failure::Server("expect END".into_string()));
            }
            let v = Item { key : mess[1].clone(),flag: FromStr::from_str(mess[2].as_slice()).unwrap(),val:next };
            Ok(Response::Value(v))
        }
        else {
            Err(Failure::Server("invalid response".into_string()))
        }

    }

}

impl Client {

    pub fn new(addr:&str) -> McResult<Client> {
        let ad : Option<SocketAddr> = FromStr::from_str(addr);
        match ad {
            Some(addr) => {
                Ok(Client{addr:addr,
                    conn:try!(TcpStream::connect_timeout(addr,Duration::seconds(1)))
                    })
            },
            None => {
                Err(Failure::Client("invalid addr".into_string()))
            }
        }
    }

    pub fn set(&mut self,key:&str,flag:u16,expire:uint,data:&str) -> McResult<Response> {
        if key.contains_char(' ') || key.contains_char('\n') || key.contains_char('\r') {
            return Err(Failure::Client("invalid key".into_string()));
        }
        let cmd = format_args!(std::fmt::format,"set {} {} {} {}\r\n{}\r\n",key,flag,expire,data.as_slice().as_bytes().len(),data);
        try!(self.send(cmd.as_slice().as_bytes()));
        self.parse()
    }

    pub fn get(&mut self,key:&str) -> McResult<Response> {
        if key.contains_char(' ') || key.contains_char('\n') || key.contains_char('\r') {
            return Err(Failure::Client("invalid key".into_string()));
        }
        let cmd = format_args!(std::fmt::format,"get {}\r\n",key);
        try!(self.send(cmd.as_slice().as_bytes()));
        self.parse()
    }

    pub fn del(&mut self,key:&str) -> McResult<Response> {
        if key.contains_char(' ') || key.contains_char('\n') || key.contains_char('\r') {
            return Err(Failure::Client("invalid key".into_string()));
        }
        let cmd = format_args!(std::fmt::format,"delete {} 0\r\n",key);
        try!(self.send(cmd.as_slice().as_bytes()));
        self.parse()
    }

    fn send(&mut self,bytes:&[u8]) -> McResult<()> {
        let mut conn = &mut self.conn;
        Ok(try!(conn.write(bytes)))
    }

    fn parse(&mut self) -> McResult<Response> {
        let mut parser = Parser::new( &mut self.conn as &mut Reader );
        Ok(try!(parser.parse_value()))
    }

}

