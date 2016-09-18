use super::{ParserError, ParserResult};
use super::primitives::{PrimitiveIterator, U1, U2, U4};

use std::ops::Deref;
use std::rc::Rc;

macro_rules! generate_constant_pool_retrieval_method {
    ($variant_name:ident, $struct_name:ident, $method_name:ident) => {
        pub fn $method_name(index: usize,
                            constant_pool: &Vec<ConstantPoolItem>)
            -> ParserResult<Rc<$struct_name>> {
                let actual_index = ConstantPoolItem::shift_index(index);

                if let Some(item) = constant_pool.get(actual_index) {
                    match item {
                        &ConstantPoolItem::$variant_name(ref val) => return Ok(val.clone()),
                        item @ _ => {
                            let name = item.to_friendly_name();
                            return Err(ParserError::UnexpectedConstantPoolItem(name))
                        }
                    }
                }

                Err(ParserError::ConstantPoolIndexOutOfBounds(actual_index))
            }

    }
}

#[derive(Debug)]
pub struct ClassInfo {
    tag: U1,
    name_index: U2,
}

#[derive(Debug)]
pub struct Utf8Info {
    tag: U1,
    length: U2,
    value: String,
}

impl Deref for Utf8Info {
    type Target = str;

    fn deref(&self) -> &str {
        &self.value
    }
}

#[derive(Debug)]
pub enum ConstantPoolItem {
    Class(Rc<ClassInfo>),
    FieldOrMethodOrInterfaceMethod {
        tag: U1,
        class_index: U2,
        name_and_type_index: U2,
    },
    String { tag: U1, string_index: U2 },
    IntegerOrFloat { tag: U1, bytes: U1 },
    LongOrDouble {
        tag: U1,
        high_bytes: U4,
        low_bytes: U4,
    },
    NameAndType {
        tag: U1,
        name_index: U2,
        descriptor_index: U2,
    },
    Utf8(Rc<Utf8Info>),
    MethodHandle {
        tag: U1,
        reference_kind: U1,
        reference_index: U2,
    },
    MethodType { tag: U1, descriptor_index: U2 },
    InvokeDynamic {
        tag: U1,
        bootstrap_method_attr_index: U2,
        name_and_type_index: U2,
    },
}

impl ConstantPoolItem {
    pub fn from<T: PrimitiveIterator>(iter: &mut T) -> ParserResult<ConstantPoolItem> {
        let tag = try!(iter.next_u1());

        match tag {
            1 => {
                let length = try!(iter.next_u2());

                let mut byte_vec = vec![];
                for _ in 0..length {
                    byte_vec.push(try!(iter.next_u1()));
                }

                let value = try!(String::from_utf8(byte_vec));

                Ok(ConstantPoolItem::Utf8(Rc::new(Utf8Info {
                    tag: tag,
                    length: length,
                    value: value,
                })))
            }
            7 => {
                Ok(ConstantPoolItem::Class(Rc::new(ClassInfo {
                    tag: tag,
                    name_index: try!(iter.next_u2()),
                })))
            }
            8 => {
                Ok(ConstantPoolItem::String {
                    tag: tag,
                    string_index: try!(iter.next_u2()),
                })
            }
            9 | 10 | 11 => {
                Ok(ConstantPoolItem::FieldOrMethodOrInterfaceMethod {
                    tag: tag,
                    class_index: try!(iter.next_u2()),
                    name_and_type_index: try!(iter.next_u2()),
                })
            }
            12 => {
                Ok(ConstantPoolItem::NameAndType {
                    tag: tag,
                    name_index: try!(iter.next_u2()),
                    descriptor_index: try!(iter.next_u2()),
                })
            }
            _ => Err(ParserError::UnknownConstantPoolTag(tag)),
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn to_friendly_name(&self) -> &'static str {
        match self {
            &ConstantPoolItem::Utf8(..) => "Utf8",
            &ConstantPoolItem::Class(..) => "Class",
            &ConstantPoolItem::String { .. } => "String",
            &ConstantPoolItem::FieldOrMethodOrInterfaceMethod { .. } =>
                "Field|Method|InterfaceMethod",
            &ConstantPoolItem::NameAndType { .. } => "NameAndType",
            _ => "Not yet implemented",
        }
    }

    generate_constant_pool_retrieval_method!(Class, ClassInfo, retrieve_class_info);
    generate_constant_pool_retrieval_method!(Utf8, Utf8Info, retrieve_utf8_info);

    fn shift_index(unshifted_index: usize) -> usize {
        unshifted_index - 1 // references to the constant pool start from one
    }
}

#[derive(Debug)]
pub enum Attribute {
    Unknown {
        attribute_name: Rc<Utf8Info>,
        info: Vec<U1>,
    },
}

impl Attribute {
    pub fn from<T: PrimitiveIterator>(iter: &mut T,
                                      constant_pool: &Vec<ConstantPoolItem>)
                                      -> ParserResult<Attribute> {
        let attribute_name_index = try!(iter.next_u2());
        let attribute_name =
            try!(ConstantPoolItem::retrieve_utf8_info(attribute_name_index as usize,
                                                      constant_pool));

        let attribute_length = try!(iter.next_u4());

        match *attribute_name {
            _ => {
                let mut info = vec![];
                for _ in 0..attribute_length {
                    info.push(try!(iter.next_u1()));
                }

                Ok(Attribute::Unknown {
                    attribute_name: attribute_name,
                    info: info,
                })
            }
        }
    }
}

#[derive(Debug)]
pub struct Field {
    access_flags: U2,
    name: Rc<Utf8Info>,
    descriptor: Rc<Utf8Info>,
    attributes_count: U2,
    attributes: Vec<Attribute>,
}

impl Field {
    pub fn from<T: PrimitiveIterator>(iter: &mut T,
                                      constant_pool: &Vec<ConstantPoolItem>)
                                      -> ParserResult<Field> {
        let access_flags = try!(iter.next_u2());

        let name_index = try!(iter.next_u2());
        let name = try!(ConstantPoolItem::retrieve_utf8_info(name_index as usize, constant_pool));

        let descriptor_index = try!(iter.next_u2());
        let descriptor = try!(ConstantPoolItem::retrieve_utf8_info(descriptor_index as usize,
                                                                   constant_pool));

        let attributes_count = try!(iter.next_u2());
        let mut attributes = vec![];
        for _ in 0..attributes_count {
            attributes.push(try!(Attribute::from(iter, constant_pool)));
        }

        Ok(Field {
            access_flags: access_flags,
            name: name,
            descriptor: descriptor,
            attributes_count: attributes_count,
            attributes: attributes,
        })

    }
}

pub struct AccessFlags;

impl AccessFlags {
    pub fn is_public(access_flags: U2) -> bool {
        (access_flags & 0x0001) != 0
    }

    pub fn is_private(access_flags: U2) -> bool {
        (access_flags & 0x0002) != 0
    }

    pub fn is_protected(access_flags: U2) -> bool {
        (access_flags & 0x0004) != 0
    }

    pub fn is_static(access_flags: U2) -> bool {
        (access_flags & 0x0008) != 0
    }

    pub fn is_final(access_flags: U2) -> bool {
        (access_flags & 0x0010) != 0
    }

    pub fn is_super(access_flags: U2) -> bool {
        (access_flags & 0x0020) != 0
    }

    pub fn is_volatile(access_flags: U2) -> bool {
        (access_flags & 0x0040) != 0
    }

    pub fn is_transient(access_flags: U2) -> bool {
        (access_flags & 0x0080) != 0
    }

    pub fn is_interface(access_flags: U2) -> bool {
        (access_flags & 0x0200) != 0
    }

    pub fn is_abstract(access_flags: U2) -> bool {
        (access_flags & 0x0400) != 0
    }

    pub fn is_synthetic(access_flags: U2) -> bool {
        (access_flags & 0x1000) != 0
    }

    pub fn is_annotation(access_flags: U2) -> bool {
        (access_flags & 0x2000) != 0
    }

    pub fn is_enum(access_flags: U2) -> bool {
        (access_flags & 0x4000) != 0
    }
}
