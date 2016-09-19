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
    pub tag: U1,
    pub name_index: U2,
}

#[derive(Debug)]
pub struct FieldOrMethodOrInterfaceMethodInfo {
    pub tag: U1,
    pub class_index: U2,
    pub name_and_type_index: U2,
}

#[derive(Debug)]
pub struct StringInfo {
    pub tag: U1,
    pub string_index: U2,
}

#[derive(Debug)]
pub struct NameAndTypeInfo {
    pub tag: U1,
    pub name_index: U2,
    pub descriptor_index: U2,
}

#[derive(Debug)]
pub struct Utf8Info {
    pub tag: U1,
    pub length: U2,
    pub value: String,
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
    FieldOrMethodOrInterfaceMethod(Rc<FieldOrMethodOrInterfaceMethodInfo>),
    String(Rc<StringInfo>),
    IntegerOrFloat { tag: U1, bytes: U1 },
    LongOrDouble {
        tag: U1,
        high_bytes: U4,
        low_bytes: U4,
    },
    NameAndType(Rc<NameAndTypeInfo>),
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
                Ok(ConstantPoolItem::String(Rc::new(StringInfo {
                    tag: tag,
                    string_index: try!(iter.next_u2()),
                })))
            }
            9 | 10 | 11 => {
                Ok(ConstantPoolItem::FieldOrMethodOrInterfaceMethod(
                        Rc::new(FieldOrMethodOrInterfaceMethodInfo {
                    tag: tag,
                    class_index: try!(iter.next_u2()),
                    name_and_type_index: try!(iter.next_u2()),
                })))
            }
            12 => {
                Ok(ConstantPoolItem::NameAndType(Rc::new(NameAndTypeInfo {
                    tag: tag,
                    name_index: try!(iter.next_u2()),
                    descriptor_index: try!(iter.next_u2()),
                })))
            }
            _ => Err(ParserError::UnknownConstantPoolTag(tag)),
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn to_friendly_name(&self) -> &'static str {
        match self {
            &ConstantPoolItem::Utf8(..) => "Utf8",
            &ConstantPoolItem::Class(..) => "Class",
            &ConstantPoolItem::String(..) => "String",
            &ConstantPoolItem::FieldOrMethodOrInterfaceMethod(..) =>
                "Field|Method|InterfaceMethod",
                &ConstantPoolItem::NameAndType(..) => "NameAndType",
                _ => "Not yet implemented",
        }
    }

    pub fn retrieve_item(index: usize,
                         constant_pool: &Vec<ConstantPoolItem>)
                         -> ParserResult<&ConstantPoolItem> {
        let actual_index = Self::shift_index(index);
        return constant_pool.get(index)
            .ok_or(ParserError::ConstantPoolIndexOutOfBounds(actual_index));
    }

    generate_constant_pool_retrieval_method!(Class, ClassInfo, retrieve_class_info);
    generate_constant_pool_retrieval_method!(Utf8, Utf8Info, retrieve_utf8_info);

    // TODO: Split the enum/struct into seperate variants
    generate_constant_pool_retrieval_method!(FieldOrMethodOrInterfaceMethod,
                                             FieldOrMethodOrInterfaceMethodInfo,
                                             retrieve_field_info);
    generate_constant_pool_retrieval_method!(FieldOrMethodOrInterfaceMethod,
                                             FieldOrMethodOrInterfaceMethodInfo,
                                             retrieve_method_info);
    generate_constant_pool_retrieval_method!(FieldOrMethodOrInterfaceMethod,
                                             FieldOrMethodOrInterfaceMethodInfo,
                                             retrieve_interface_method_info);

    generate_constant_pool_retrieval_method!(String, StringInfo, retrieve_string_info);
    generate_constant_pool_retrieval_method!(NameAndType,
                                             NameAndTypeInfo,
                                             retrieve_name_and_type_info);

    fn shift_index(unshifted_index: usize) -> usize {
        unshifted_index - 1 // references to the constant pool start from one
    }
}

#[derive(Debug)]
pub struct CodeAttribute {
    pub attribute_name: Rc<Utf8Info>,
    pub max_stack: U2,
    pub max_locals: U2,
    pub code_length: U4,
    pub code: Vec<U1>,
    pub exception_table_length: U2,
    pub exception_table: Vec<ExceptionHandler>,
    pub attributes_count: U2,
    pub attributes: Vec<Attribute>,
}

impl CodeAttribute {
    pub fn from<T: PrimitiveIterator>(attribute_name: Rc<Utf8Info>,
                                      iter: &mut T,
                                      constant_pool: &Vec<ConstantPoolItem>)
                                      -> ParserResult<CodeAttribute> {
        let max_stack = try!(iter.next_u2());
        let max_locals = try!(iter.next_u2());

        let code_length = try!(iter.next_u4());
        let mut code = vec![];
        for _ in 0..code_length {
            code.push(try!(iter.next_u1()));
        }

        let exception_table_length = try!(iter.next_u2());
        let mut exception_table = vec![];
        for _ in 0..exception_table_length {
            exception_table.push(try!(ExceptionHandler::from(iter)));
        }

        let attributes_count = try!(iter.next_u2());
        let mut attributes = vec![];
        for _ in 0..attributes_count {
            attributes.push(try!(Attribute::from(iter, constant_pool)));
        }

        Ok(CodeAttribute {
            attribute_name: attribute_name,
            max_stack: max_stack,
            max_locals: max_locals,
            code_length: code_length,
            code: code,
            exception_table_length: exception_table_length,
            exception_table: exception_table,
            attributes_count: attributes_count,
            attributes: attributes,
        })
    }
}

#[derive(Debug)]
pub struct ExceptionHandler {
    pub start_pc: U2,
    pub end_pc: U2,
    pub handler_pc: U2,
    pub catch_type: U2,
}

impl ExceptionHandler {
    pub fn from<T: PrimitiveIterator>(iter: &mut T) -> ParserResult<ExceptionHandler> {
        let start_pc = try!(iter.next_u2());
        let end_pc = try!(iter.next_u2());
        let handler_pc = try!(iter.next_u2());
        let catch_type = try!(iter.next_u2());

        Ok(ExceptionHandler {
            start_pc: start_pc,
            end_pc: end_pc,
            handler_pc: handler_pc,
            catch_type: catch_type,
        })
    }
}

#[derive(Debug)]
pub enum Attribute {
    Code(Rc<CodeAttribute>),
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

        match &**attribute_name {
            "Code" => {
                Ok(Attribute::Code(Rc::new(try!(CodeAttribute::from(attribute_name,
                                                                    iter,
                                                                    constant_pool)))))
            }
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

macro_rules! generate_method_or_field_parser_impl {
    ($impl_name:ident) => {
        impl $impl_name {
            pub fn from<T: PrimitiveIterator>(iter: &mut T,
                                              constant_pool: &Vec<ConstantPoolItem>)
                -> ParserResult<$impl_name> {
                    let access_flags = try!(iter.next_u2());

                    let name_index = try!(iter.next_u2());
                    let name = try!(ConstantPoolItem::retrieve_utf8_info(name_index as usize,
                                                                         constant_pool));

                    let descriptor_index = try!(iter.next_u2());
                    let descriptor = try!(ConstantPoolItem::retrieve_utf8_info(
                            descriptor_index as usize,
                            constant_pool));

                    let attributes_count = try!(iter.next_u2());
                    let mut attributes = vec![];
                    for _ in 0..attributes_count {
                        attributes.push(Rc::new(try!(Attribute::from(iter, constant_pool))));
                    }

                    Ok($impl_name {
                        access_flags: access_flags,
                        name: name,
                        descriptor: descriptor,
                        attributes_count: attributes_count,
                        attributes: attributes,
                    })

                }
        }
    }
}

#[derive(Debug)]
pub struct Field {
    pub access_flags: U2,
    pub name: Rc<Utf8Info>,
    pub descriptor: Rc<Utf8Info>,
    pub attributes_count: U2,
    pub attributes: Vec<Rc<Attribute>>,
}

#[derive(Debug)]
pub struct Method {
    pub access_flags: U2,
    pub name: Rc<Utf8Info>,
    pub descriptor: Rc<Utf8Info>,
    pub attributes_count: U2,
    pub attributes: Vec<Rc<Attribute>>,
}

generate_method_or_field_parser_impl!(Field);
generate_method_or_field_parser_impl!(Method);

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

    pub fn is_bridge(access_flags: U2) -> bool {
        (access_flags & 0x0040) != 0
    }

    pub fn is_transient(access_flags: U2) -> bool {
        (access_flags & 0x0080) != 0
    }

    pub fn is_varargs(access_flags: U2) -> bool {
        (access_flags & 0x0080) != 0
    }

    pub fn is_native(access_flags: U2) -> bool {
        (access_flags & 0x0100) != 0
    }

    pub fn is_interface(access_flags: U2) -> bool {
        (access_flags & 0x0200) != 0
    }

    pub fn is_abstract(access_flags: U2) -> bool {
        (access_flags & 0x0400) != 0
    }

    pub fn is_strict(access_flags: U2) -> bool {
        (access_flags & 0x0800) != 0
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
