use components::{Attribute, ConstantPoolItem, ConstantPoolResolver, Field, Method, Utf8Info};
use primitives::{PrimitiveIterator, U1, U2, U4};

use std::fs::File;
use std::io::{Error as IoError, Read};
use std::rc::Rc;
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

macro_rules! rc_populate_vec {
    ($length:ident, $supplier:expr) => {
        {
            let mut temp_vec = vec![];
            for _ in 0..$length {
                temp_vec.push(Rc::new(try!($supplier)));
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
    pub fields: Vec<Rc<Field>>,
    pub methods_count: U2,
    pub methods: Vec<Rc<Method>>,
    pub attributes_count: U2,
    pub attributes: Vec<Rc<Attribute>>,
}

impl ClassFile {
    pub fn from(file: File) -> ParserResult<ClassFile> {
        let mut bytes = file.bytes();

        let magic = try!(bytes.next_u4());
        let minor_version = try!(bytes.next_u2());
        let major_version = try!(bytes.next_u2());

        let constant_pool_count = try!(bytes.next_u2());
        let actual_constant_pool_count = constant_pool_count - 1;
        let constant_pool = try!(Self::build_constant_pool(actual_constant_pool_count, &mut bytes));

        let access_flags = try!(bytes.next_u2());
        let this_class = try!(bytes.next_u2());
        let super_class = try!(bytes.next_u2());

        let interfaces_count = try!(bytes.next_u2());
        let interfaces = populate_vec!(interfaces_count, bytes.next_u2());

        let fields_count = try!(bytes.next_u2());
        let fields = rc_populate_vec!(fields_count, Field::from(&mut bytes, &constant_pool));

        let methods_count = try!(bytes.next_u2());
        let methods = rc_populate_vec!(methods_count, Method::from(&mut bytes, &constant_pool));

        let attributes_count = try!(bytes.next_u2());
        let attributes = rc_populate_vec!(attributes_count,
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

    pub fn classname(&self) -> ParserResult<Rc<Utf8Info>> {
        let this_class = self.this_class;
        let constant_pool = &self.constant_pool;

        let class_info = try!(ConstantPoolItem::retrieve_class_info(this_class, constant_pool));
        let utf8_info = try!(ConstantPoolItem::retrieve_utf8_info(class_info.name_index,
                                                                  constant_pool));

        Ok(utf8_info)
    }

    pub fn maybe_resolve_main_method(&self) -> Option<Rc<Method>> {
        self.maybe_resolve_method(&"main")
    }

    pub fn maybe_resolve_method(&self, name: &str) -> Option<Rc<Method>> {
        for method in &self.methods {
            if method.name.eq(name) {
                return Some(method.clone());
            }
        }

        None
    }

    pub fn constant_pool_resolver(&self) -> ConstantPoolResolver {
        ConstantPoolResolver { constant_pool: &self.constant_pool }
    }

    fn build_constant_pool<T: PrimitiveIterator>(constant_pool_count: U2,
                                                 iter: &mut T)
                                                 -> ParserResult<Vec<ConstantPoolItem>> {
        let mut should_skip = false;
        let mut constant_pool = vec![];

        for _ in 0..constant_pool_count {
            if should_skip {
                should_skip = false;
                continue;
            }

            let constant_pool_item = try!(ConstantPoolItem::from(iter));
            match constant_pool_item {
                item @ ConstantPoolItem::Long(..) |
                item @ ConstantPoolItem::Double(..) => {
                    should_skip = true;
                    constant_pool.push(item);
                    constant_pool.push(ConstantPoolItem::Empty);
                }
                item @ _ => constant_pool.push(item),
            }
        }

        Ok(constant_pool)
    }
}

#[cfg(test)]
mod tests {

    extern crate spectral;

    use self::spectral::prelude::*;

    use super::ClassFile;
    use super::components::{Attribute, AccessFlags, ConstantPoolItem};
    use super::primitives::U2;

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

        assert_that(&ConstantPoolItem::retrieve_class_info(this_class, &constant_pool)).is_ok();
        assert_that(&ConstantPoolItem::retrieve_class_info(super_class, &constant_pool)).is_ok();
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

        let method_attributes: Vec<_> =
            classfile.methods.iter().flat_map(|val| &val.attributes).collect();
        asserting("at least one method has the code attribute")
            .that(&method_attributes)
            .matching_contains(|val| match ***val {
                Attribute::Code(..) => true,
                _ => false,
            });
    }

    #[test]
    fn can_successfully_parse_class_attributes() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.attributes_count).is_equal_to(&1);
        assert_that(&classfile.attributes).has_length(1);
    }

    #[test]
    fn can_resolve_main_method_if_present() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        assert_that(&classfile.maybe_resolve_main_method()).is_some();
    }

    #[test]
    fn can_retrieve_classname() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        let classname = classfile.classname().unwrap();
        assert_that(&classname.to_string()).is_equal_to(&"HelloWorld".to_string());
    }

    #[test]
    fn can_resolve_string_from_constant_pool() {
        let test_file = open_test_resource("classfile/HelloWorld.class");
        let classfile = ClassFile::from(test_file).unwrap();

        let resolver = classfile.constant_pool_resolver();

        let string_indexes: Vec<U2> = classfile.constant_pool
            .iter()
            .enumerate()
            .filter(|&(_, item)| {
                return match item {
                    &ConstantPoolItem::String(..) => true,
                    _ => false,
                };
            })
            .map(|(i, _)| (i + 1) as U2)
            .collect();

        let strings =
            string_indexes.iter().map(|i| resolver.resolve_string_constant(*i).unwrap()).collect();

        assert_that(&strings)
            .has_length(1)
            .contains(&"hello world".to_string());
    }

    fn open_test_resource(resource_path: &str) -> File {
        let mut file_path = PathBuf::from(MANIFEST_DIR);
        file_path.push("test-resources/");
        file_path.push(resource_path);

        File::open(file_path).unwrap()
    }

}
