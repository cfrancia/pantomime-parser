use std::io::{Bytes, Error as IoError, ErrorKind as IoErrorKind, Read, Result as IoResult};
use std::iter::Iterator;

pub type U1 = u8;
pub type U2 = u16;
pub type U4 = u32;

pub trait PrimitiveIterator: Iterator<Item = IoResult<u8>> {
    fn next_u1(&mut self) -> IoResult<U1> {
        self.next().as_result_or(new_eof_error())
    }

    fn next_u2(&mut self) -> IoResult<U2> {
        let first = try!(self.next_u1()) as U2;
        let second = try!(self.next_u1()) as U2;

        Ok((first << 8) + (second << 0))
    }

    fn next_u4(&mut self) -> IoResult<U4> {
        let first = try!(self.next_u2()) as U4;
        let second = try!(self.next_u2()) as U4;

        Ok((first << 16) + (second << 0))
    }
}

impl<R: Read> PrimitiveIterator for Bytes<R> {}

fn new_eof_error() -> IoError {
    IoError::new(IoErrorKind::UnexpectedEof,
                 "tried to read byte but end of file reached")
}

trait FromOptionResult<T, E> {
    fn as_result_or(self, error: E) -> Result<T, E>;
}

impl<T, E> FromOptionResult<T, E> for Option<Result<T, E>> {
    fn as_result_or(self, error: E) -> Result<T, E> {
        self.unwrap_or(Err(error))
    }
}
