#![feature(globs)]

use std::io::*;
use std::error::*;
use std::io::net::ip::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use std::cell::RefCell;

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
    val:String
}

#[deriving(Show)]
pub enum Response {
    Stored,
    NotStored,
    InvalidCmd,
    Deleted,
    NotFound,
    ClientErr(String),
    ServerErr(String),
    Value(Item)
}

pub struct Parser<T> {
    reader : T
}

impl <'a,T:Reader> Parser<T> {

    pub fn new(reader: T) -> Parser<T> {
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

    fn read_line(&mut self) -> McResult<Vec<u8>> {
        let mut rv = vec![];

        loop {
            let b = try!(self.reader.read_byte());
            match b as char {
                '\n' => { break; }
                '\r' => {
                    try!(self.expect_char('\n'));
                    break;
                },
                _ => { rv.push(b) }
            };
        }

        Ok(rv)
    }

    fn read_string_line(&mut self) -> McResult<String> {
        match String::from_utf8(try!(self.read_line())) {
            Err(_) => {
                Err(Failure::Server("Expect string,Invalid byte in response".into_string()))
            }
            Ok(value) => Ok(value)
        }
    }

    pub fn parse_value(&mut self) -> McResult<Response> {
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
            let next = self.read_string_line().unwrap();
            let end = self.read_string_line().unwrap();
            if end.as_slice() != "END" {
                return Err(Failure::Server("expect END".into_string()));
            }
            let mut mess : Vec<String> = vec![];
            for s in line.split_str(" ") {
                mess.push(String::from_str(s))
            }
            let v = Item { key : mess[0].clone(),flag: FromStr::from_str(mess[1].as_slice()).unwrap_or(0u16),val:next };
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
        let cmd = format_args!(std::fmt::format,"set {} {} {} {}\r\n{}\r\n",key,flag,expire,data.as_slice().as_bytes().len(),data);
        try!(self.send(cmd.as_slice().as_bytes()));
        self.parse()
    }

    pub fn get(&mut self,key:&str) -> McResult<Response> {
        let cmd = format_args!(std::fmt::format,"get {}\r\n",key);
        try!(self.send(cmd.as_slice().as_bytes()));
        self.parse()
    }

    pub fn del(&mut self,key:&str) -> McResult<Response> {
        let cmd = format_args!(std::fmt::format,"delete {} 0\r\n",key);
        try!(self.send(cmd.as_slice().as_bytes()));
        self.parse()
    }

    fn send(&mut self,bytes:&[u8]) -> McResult<()> {
        let mut conn = &mut self.conn;
        let w = try!(conn.write(bytes));
        Ok(w)
    }

    fn parse(&mut self) -> McResult<Response> {
        let mut parser = Parser::new( &mut self.conn as &mut Reader );
        let res = try!(parser.parse_value());
        Ok(res)
    }

}

