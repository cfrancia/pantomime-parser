use components::ConstantPoolItem;
use primitives::{PrimitiveIterator, U1, U2, U4};

use std::fs::File;
use std::io::{Error as IoError, Read};
use std::string::FromUtf8Error;

pub mod components;
pub mod primitives;

pub type ParserResult<T> = Result<T, ParserError>;

#[derive(Debug)]
pub enum ParserError {
    UnknownConstantPoolTag(U1),
    InvalidUtf8(FromUtf8Error),
    Io(IoError),
}

impl From<IoError> for ParserError {
    fn from(error: IoError) -> ParserError {
        ParserError::Io(error)
    }
}

impl From<FromUtf8Error> for ParserError {
    fn from(error: FromUtf8Error) -> ParserError {
        ParserError::InvalidUtf8(error)
    }
}

#[derive(Debug)]
pub struct ClassFile {
    pub magic: U4,
    pub minor_version: U2,
    pub major_version: U2,
    pub constant_pool_count: U2,
    pub constant_pool: Vec<ConstantPoolItem>,
}

impl ClassFile {
    pub fn from(file: File) -> ParserResult<ClassFile> {
        let mut bytes = file.bytes();

        let magic = try!(bytes.next_u4());
        let minor_version = try!(bytes.next_u2());
        let major_version = try!(bytes.next_u2());

        let constant_pool_count = try!(bytes.next_u2());
        let mut constant_pool = vec![];
        for _ in 1..constant_pool_count {
            constant_pool.push(try!(ConstantPoolItem::from(&mut bytes)));
        }

        Ok(ClassFile {
            magic: magic,
            minor_version: minor_version,
            major_version: major_version,
            constant_pool_count: constant_pool_count,
            constant_pool: constant_pool,
        })
    }
}

#[cfg(test)]
mod tests {

    extern crate spectral;

    use self::spectral::prelude::*;

    use super::ClassFile;

    use std::fs::File;
    use std::path::PathBuf;

    const MANIFEST_DIR: &'static str = env!("CARGO_MANIFEST_DIR");

    #[test]
    fn can_successfully_parse_magic() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.magic).is_equal_to(&0xCAFEBABE);
    }

    #[test]
    fn can_successfully_parse_version() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.minor_version).is_equal_to(&0);
        assert_that(&classfile.major_version).is_equal_to(&52);
    }

    #[test]
    fn can_successfully_parse_constant_pool() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.constant_pool_count).is_equal_to(&26);
        assert_that(&classfile.constant_pool).has_length(25);
    }


    fn open_test_resource(resource_path: &str) -> File {
        let mut file_path = PathBuf::from(MANIFEST_DIR);
        file_path.push("test-resources/");
        file_path.push(resource_path);

        File::open(file_path).unwrap()
    }

}