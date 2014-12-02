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

pub struct Parser<T> {
    reader : T
}

impl <T> Parser<T> where T:Reader {
    
    pub fn new(reader: T) -> Parser<T> {
        Parser { reader: reader }
    }

    #[inline]
    fn expect_char(&mut self, refchar: char) -> McResult<()> {
        if try!(self.reader.read_byte()) as char == refchar {
            Ok(())
        } else {
            Err(FromError::from_error(ClientError{desc:"Invalid byte in response".into_string()}))
        }
    }

    #[inline]
    fn expect_newline(&mut self) -> McResult<()> {
        match try!(self.reader.read_byte()) as char {
            '\n' => Ok(()),
            '\r' => self.expect_char('\n'),
            _ => Err(FromError::from_error(ClientError{desc:"Expect new line,Invalid byte in response".into_string()}))
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
                Err(FromError::from_error(ClientError{desc:"Expect string,Invalid byte in response".into_string()}))
            }
            Ok(value) => Ok(value)
        }
    }

    pub fn parse_value(&mut self) -> McResult<Response> {
        let ret = self.read_string_line().unwrap();
        let line = ret.as_slice();
        if line.len() < 5 {
                return Err(FromError::from_error(ClientError{desc:"Expect more,Invalid byte in response".into_string()}));
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
        else if line.starts_with("CLIENT_ERROR") {
            let err = line.slice(12,line.len()-4).into_string();
            Ok(Response::ClientErr(err))
        }
        else if line.starts_with("SERVER_ERROR") {
            let err = line.slice(12,line.len()-4).into_string();
            Ok(Response::ServerErr(err))
        }
        else {
             Err(FromError::from_error(ClientError{desc:"invalid response".into_string()}))
        }

    }

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

    pub fn set(&mut self,key:&str,flag:u16,expire:uint,data:&str) -> McResult<Response> {
        let cmd = format_args!(std::fmt::format,"set {} {} {} {}\r\n{}\r\n",key,flag,expire,data.as_slice().as_bytes().len(),data);

        let mut conn = self.conn.borrow_mut();
        conn.set_timeout(Some(1000u64));

        try!(conn.write(cmd.as_slice().as_bytes()));
        let mut parser = Parser::new(&mut conn as &mut Reader);
        let res = try!(parser.parse_value());
        Ok(res)
   }
}

