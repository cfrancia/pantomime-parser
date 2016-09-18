use components::{Attribute, ConstantPoolItem, Field, Method};
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
    UnexpectedConstantPoolItem(&'static str),
    ConstantPoolIndexOutOfBounds(usize),
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

macro_rules! populate_vec {
    ($length:ident, $supplier:expr) => {
        {
            let mut temp_vec = vec![];
            for _ in 0..$length {
                temp_vec.push(try!($supplier));
            }

            temp_vec
        }
    }
}

#[derive(Debug)]
pub struct ClassFile {
    pub magic: U4,
    pub minor_version: U2,
    pub major_version: U2,
    pub constant_pool_count: U2,
    pub constant_pool: Vec<ConstantPoolItem>,
    pub access_flags: U2,
    pub this_class: U2,
    pub super_class: U2,
    pub interfaces_count: U2,
    pub interfaces: Vec<U2>,
    pub fields_count: U2,
    pub fields: Vec<Field>,
    pub methods_count: U2,
    pub methods: Vec<Method>,
    pub attributes_count: U2,
    pub attributes: Vec<Attribute>,
}

impl ClassFile {
    pub fn from(file: File) -> ParserResult<ClassFile> {
        let mut bytes = file.bytes();

        let magic = try!(bytes.next_u4());
        let minor_version = try!(bytes.next_u2());
        let major_version = try!(bytes.next_u2());

        let constant_pool_count = try!(bytes.next_u2());
        let actual_constant_pool_count = constant_pool_count - 1;
        let constant_pool = populate_vec!(actual_constant_pool_count,
                                          ConstantPoolItem::from(&mut bytes));

        let access_flags = try!(bytes.next_u2());
        let this_class = try!(bytes.next_u2());
        let super_class = try!(bytes.next_u2());

        let interfaces_count = try!(bytes.next_u2());
        let interfaces = populate_vec!(interfaces_count, bytes.next_u2());

        let fields_count = try!(bytes.next_u2());
        let fields = populate_vec!(fields_count, Field::from(&mut bytes, &constant_pool));

        let methods_count = try!(bytes.next_u2());
        let methods = populate_vec!(methods_count, Method::from(&mut bytes, &constant_pool));

        let attributes_count = try!(bytes.next_u2());
        let attributes = populate_vec!(attributes_count,
                                       Attribute::from(&mut bytes, &constant_pool));

        Ok(ClassFile {
            magic: magic,
            minor_version: minor_version,
            major_version: major_version,
            constant_pool_count: constant_pool_count,
            constant_pool: constant_pool,
            access_flags: access_flags,
            this_class: this_class,
            super_class: super_class,
            interfaces_count: interfaces_count,
            interfaces: interfaces,
            fields_count: fields_count,
            fields: fields,
            methods_count: methods_count,
            methods: methods,
            attributes_count: attributes_count,
            attributes: attributes,
        })
    }
}

#[cfg(test)]
mod tests {

    extern crate spectral;

    use self::spectral::prelude::*;

    use super::ClassFile;
    use super::components::{AccessFlags, ConstantPoolItem};

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

    #[test]
    fn can_successfully_parse_access_flags() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        let access_flags = classfile.access_flags;
        asserting("class is public")
            .that(&access_flags)
            .matches(|val| AccessFlags::is_public(*val));
        asserting("class is super").that(&access_flags).matches(|val| AccessFlags::is_super(*val));
    }

    #[test]
    fn can_successfully_parse_class_references() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        let this_class = classfile.this_class;
        let super_class = classfile.super_class;
        let constant_pool = classfile.constant_pool;

        assert_that(&ConstantPoolItem::retrieve_class_info(this_class as usize, &constant_pool))
            .is_ok();
        assert_that(&ConstantPoolItem::retrieve_class_info(super_class as usize, &constant_pool))
            .is_ok();
    }

    #[test]
    fn can_successfully_parse_interfaces() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.interfaces_count).is_equal_to(&0);
        assert_that(&classfile.interfaces).has_length(0);
    }

    #[test]
    fn can_successfully_parse_fields() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.fields_count).is_equal_to(&0);
        assert_that(&classfile.fields).has_length(0);
    }

    #[test]
    fn can_successfully_parse_methods() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.methods_count).is_equal_to(&3);
        assert_that(&classfile.methods)
            .has_length(3)
            .mapped_contains(|val| &**val.name, &"<init>")
            .mapped_contains(|val| &**val.name, &"main")
            .mapped_contains(|val| &**val.name, &"println");
    }

    #[test]
    fn can_successfully_parse_class_attributes() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.attributes_count).is_equal_to(&1);
        assert_that(&classfile.attributes).has_length(1);
    }

    fn open_test_resource(resource_path: &str) -> File {
        let mut file_path = PathBuf::from(MANIFEST_DIR);
        file_path.push("test-resources/");
        file_path.push(resource_path);

        File::open(file_path).unwrap()
    }

}
