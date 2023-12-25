extern crate ringhopper_definitions;

use std::fmt::Write;
use std::borrow::Cow;

use ringhopper_definitions::{load_all_definitions, Struct, NamedObject, Enum, Bitfield, StructFieldType, ObjectType};

use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_ringhopper_structs(_: TokenStream) -> TokenStream {
    let definitions = load_all_definitions();
    let mut stream = TokenStream::default();

    for (_, obj) in definitions.objects {
        stream.extend(obj.to_token_stream());
    }

    stream
}

trait ToTokenStream {
    fn to_token_stream(&self) -> TokenStream;
}

impl ToTokenStream for NamedObject {
    fn to_token_stream(&self) -> TokenStream {
        match self {
            Self::Struct(s) => s.to_token_stream(),
            Self::Bitfield(b) => b.to_token_stream(),
            Self::Enum(e) => e.to_token_stream()
        }
    }
}

impl ToTokenStream for Struct {
    fn to_token_stream(&self) -> TokenStream {
        let struct_name = &self.name;
        let mut fields = String::new();
        for i in &self.fields {
            let field_type = match &i.field_type {
                StructFieldType::Padding(_) => continue,
                StructFieldType::Object(o) => match o {
                    ObjectType::Angle => "Angle".to_owned(),
                    ObjectType::ColorARGBFloat => "ColorARGBFloat".to_owned(),
                    ObjectType::ColorRGBFloat => "ColorRGBFloat".to_owned(),
                    ObjectType::ColorARGBInt => "ColorARGBInt".to_owned(),
                    ObjectType::Data => "Data".to_owned(),
                    ObjectType::Euler2D => "Euler2D".to_owned(),
                    ObjectType::Euler3D => "Euler3D".to_owned(),
                    ObjectType::F32 => "f32".to_owned(),
                    ObjectType::I16 => "i16".to_owned(),
                    ObjectType::I32 => "i32".to_owned(),
                    ObjectType::I8 => "i8".to_owned(),
                    ObjectType::Index => "Index".to_owned(),
                    ObjectType::Matrix3x3 => "Matrix3x3".to_owned(),
                    ObjectType::Plane2D => "Plane2D".to_owned(),
                    ObjectType::Plane3D => "Plane3D".to_owned(),
                    ObjectType::Address => "Address".to_owned(),
                    ObjectType::Quaternion => "Quaternion".to_owned(),
                    ObjectType::String32 => "String32".to_owned(),
                    ObjectType::TagID => "ID".to_owned(),
                    ObjectType::TagReference(_) => "TagReference".to_owned(),
                    ObjectType::U16 => "u16".to_owned(),
                    ObjectType::U32 => "u32".to_owned(),
                    ObjectType::U8 => "u8".to_owned(),
                    ObjectType::Vector2D => "Vector2D".to_owned(),
                    ObjectType::Vector3D => "Vector3D".to_owned(),
                    ObjectType::NamedObject(o) => o.to_owned(),
                    ObjectType::Reflexive(o) => format!("Reflexive<{o}>"),
                    ObjectType::ScenarioScriptNodeValue => "ScenarioScriptNodeValue".to_owned(),
                    ObjectType::Vector2DInt => "Vector2DInt".to_owned(),
                    ObjectType::Rectangle => "Rectangle".to_owned()
                }
            };

            let field_name = safe_str(&i.name);

            writeln!(&mut fields, "pub {field_name}: {field_type},").unwrap();
        }

        let structure = format!("
        #[derive(Clone, PartialEq, Default, Debug)]
        pub struct {struct_name} {{
            {fields}
        }}").parse::<TokenStream>().unwrap();

        let structure_size = self.size;

        let functions = format!("impl TagData for {struct_name} {{
            fn size() -> usize {{
                {structure_size}
            }}

            fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {{
                todo!()
            }}

            fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {{
                todo!()
            }}
        }}").parse::<TokenStream>().unwrap();

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);

        tokens
    }
}

impl ToTokenStream for Enum {
    fn to_token_stream(&self) -> TokenStream {
        macro_rules! writeln_for_each_field {
            ($($fmt:expr)*) => {{
                let mut out = String::new();
                for i in &self.options {
                    writeln!(&mut out, $($fmt)*, field=camel_case(&i.name), value=i.value).unwrap();
                }
                out
            }};
        }

        let struct_name = &self.name;
        let fields = writeln_for_each_field!("{field}, // {value}");
        let read_in = writeln_for_each_field!("{value} => Ok(Self::{field}),");

        let structure = format!("
        #[derive(Copy, Clone, PartialEq, Default, Debug)]
        #[repr(u16)]
        pub enum {struct_name} {{
            #[default]
            {fields}
        }}").parse::<TokenStream>().unwrap();

        let functions = format!("
        impl TagDataSimplePrimitive for {struct_name} {{
            fn size() -> usize {{
                <u16 as TagDataSimplePrimitive>::size()
            }}
            fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {{
                let input = u16::read::<B>(data, at, struct_end)?;
                match input {{
                    {read_in}
                    _ => Err(Error::InvalidEnum)
                }}
            }}
            fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {{
                (*self as u16).write::<B>(data, at, struct_end)
            }}
        }}").parse::<TokenStream>().unwrap();

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);

        tokens
    }
}

impl ToTokenStream for Bitfield {
    fn to_token_stream(&self) -> TokenStream {
        macro_rules! writeln_for_each_field {
            ($($fmt:expr)*) => {{
                let mut out = String::new();
                for i in &self.fields {
                    writeln!(&mut out, $($fmt)*, field=safe_str(&i.name), value=i.value).unwrap();
                }
                out
            }};
        }

        let struct_name = &self.name;

        let fields = writeln_for_each_field!("pub {field}: bool, // {value}");
        let structure = format!("
        #[derive(Copy, Clone, PartialEq, Default, Debug)]
        pub struct {struct_name} {{
            {fields}
        }}").parse::<TokenStream>().unwrap();

        let width = self.width;
        let write_out = writeln_for_each_field!("output |= self.{field} as u{width} * {value};");
        let read_in = writeln_for_each_field!("{field}: (input & {value}) != 0,");

        let functions = format!("
        impl TagDataSimplePrimitive for {struct_name} {{
            fn size() -> usize {{
                <u{width} as TagDataSimplePrimitive>::size()
            }}
            fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {{
                let input = u{width}::read::<B>(data, at, struct_end)?;
                Ok(Self {{
                    {read_in}
                }})
            }}
            fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {{
                let mut output = 0u{width};
                {write_out}
                output.write::<B>(data, at, struct_end)
            }}
        }}").parse::<TokenStream>().unwrap();

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);

        tokens
    }
}

fn camel_case(string: &str) -> String {
    let safe = safe_str(string);

    let mut result = String::with_capacity(safe.len());
    let mut capital = true;

    for c in safe.chars() {
        if c == '_' {
            capital = true;
            if result.is_empty() {
                result.push('_');
            }
            continue;
        }

        if capital {
            capital = false;
            result.push(c.to_ascii_uppercase());
            continue;
        }

        result.push(c);
    }

    result
}

fn safe_str(string: &str) -> Cow<str> {
    let mut string = Cow::Borrowed(string);

    if string.contains("'") {
        string = Cow::Owned(string.replace("'", ""));
    }

    if string.contains(" ") {
        string = Cow::Owned(string.replace(" ", "_"));
    }

    if string.contains("-") {
        string = Cow::Owned(string.replace("-", "_"));
    }

    if string.contains("(") {
        string = Cow::Owned(string.replace("(", ""));
    }

    if string.contains(")") {
        string = Cow::Owned(string.replace(")", ""));
    }

    if string.chars().next().unwrap().is_numeric() {
        string = Cow::Owned(format!("_{string}"));
    }

    if string == "type" {
        string = Cow::Borrowed("_type");
    }

    if string == "loop" {
        string = Cow::Borrowed("_loop");
    }

    string
}
