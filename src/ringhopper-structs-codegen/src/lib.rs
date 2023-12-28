extern crate ringhopper_definitions;

use std::fmt::Write;
use std::borrow::Cow;

use ringhopper_definitions::{load_all_definitions, SizeableObject, Struct, NamedObject, Enum, Bitfield, StructFieldType, ObjectType, ParsedDefinitions, FieldCount, TagGroup};

use proc_macro::TokenStream;

#[proc_macro]
pub fn generate_ringhopper_structs(_: TokenStream) -> TokenStream {
    let definitions = load_all_definitions();
    let mut stream = TokenStream::default();

    for (_, obj) in &definitions.objects {
        stream.extend(obj.to_token_stream(&definitions));
    }

    let mut read_any_tag_lines = String::new();
    for (group_name, group) in &definitions.groups {
        stream.extend(group.to_token_stream(&definitions));

        let struct_name = &group.struct_name;
        let group_name_fixed = camel_case(&group_name);
        writeln!(read_any_tag_lines, "TagGroup::{group_name_fixed} => b(TagFile::read_tag_from_file_buffer::<{struct_name}>(file, ParseStrictness::Relaxed)),").unwrap();
    }

    stream.extend(format!("
    /// Read the tag file buffer.
    ///
    /// Returns `Err` if the tag data is invalid, corrupt, or does not correspond to any known tag group.
    pub fn read_any_tag_from_file_buffer(file: &[u8], strictness: ParseStrictness) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {{
        let (header, _) = TagFile::load_header_and_data(file, strictness)?;

        fn b<T: PrimaryTagStruct + 'static>(what: RinghopperResult<T>) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {{
            what.map(|b| Box::<T>::new(b) as Box<dyn PrimaryTagStructDyn>)
        }}

        match header.group {{
            {read_any_tag_lines}
            _ => Err(Error::TagGroupUnimplemented)
        }}
    }}").parse::<TokenStream>());

    stream
}

trait ToTokenStream {
    fn to_token_stream(&self, definitions: &ParsedDefinitions) -> TokenStream;
}

impl ToTokenStream for NamedObject {
    fn to_token_stream(&self, definitions: &ParsedDefinitions) -> TokenStream {
        match self {
            Self::Struct(s) => s.to_token_stream(definitions),
            Self::Bitfield(b) => b.to_token_stream(definitions),
            Self::Enum(e) => e.to_token_stream(definitions)
        }
    }
}

impl ToTokenStream for Struct {
    fn to_token_stream(&self, definitions: &ParsedDefinitions) -> TokenStream {
        let struct_name = &self.name;
        let mut fields = String::new();

        let mut fields_with_types: Vec<String> = Vec::new();
        let mut fields_with_names: Vec<Cow<str>> = Vec::new();
        let mut fields_with_sizes: Vec<usize> = Vec::new();
        let mut fields_read_from_tags: Vec<bool> = Vec::new();
        let mut fields_read_from_caches: Vec<bool> = Vec::new();

        let field_count = self.fields.len();

        for i in 0..field_count {
            let field = &self.fields[i];
            let field_type = match &field.field_type {
                StructFieldType::Padding(n) => format!("Padding<[u8; {n}]>"),
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

            let field_type = match field.count {
                FieldCount::Array(n) => format!("[{field_type}; {n}]"),
                FieldCount::Bounds => format!("Bounds<{field_type}>"),
                FieldCount::One => field_type
            };

            if let StructFieldType::Padding(_) = &field.field_type {
                fields_with_names.push(Default::default());
                fields_read_from_tags.push(false);
                fields_read_from_caches.push(false);
            }
            else {
                let field_name = safe_str(&field.name);
                writeln!(&mut fields, "pub {field_name}: {field_type},").unwrap();
                fields_with_names.push(field_name);
                fields_read_from_tags.push(!field.flags.cache_only);
                fields_read_from_caches.push(!field.flags.non_cached);
            }

            fields_with_types.push(field_type);
            fields_with_sizes.push(field.size(definitions));
        }

        let structure = format!("
        #[derive(Clone, PartialEq, Default, Debug)]
        pub struct {struct_name} {{
            {fields}
        }}").parse::<TokenStream>().unwrap();

        let structure_size = self.size;

        let mut write_out = String::new();
        let mut read_in = String::new();

        let mut field_list = String::new();
        let mut getter = String::new();
        let mut getter_mut = String::new();

        for i in 0..field_count {
            let length = &fields_with_sizes[i];

            if fields_read_from_tags[i] {
                let field_name = &fields_with_names[i];
                let field_type = &fields_with_types[i];

                writeln!(&mut write_out, "self.{field_name}.write_to_tag_file(data, _pos, struct_end)?;").unwrap();
                writeln!(&mut read_in, "output.{field_name} = <{field_type}>::read_from_tag_file(data, _pos, struct_end, extra_data_cursor)?;").unwrap();
                write!(&mut field_list, "\"{field_name}\",").unwrap();

                writeln!(&mut getter, "\"{field_name}\" => Some(&self.{field_name}),").unwrap();
                writeln!(&mut getter_mut, "\"{field_name}\" => Some(&mut self.{field_name}),").unwrap();
            }

            writeln!(&mut write_out, "let _pos = _pos.add_overflow_checked({length})?;").unwrap();
            writeln!(&mut read_in, "let _pos = _pos.add_overflow_checked({length})?;").unwrap();
        }

        let functions = format!("impl TagData for {struct_name} {{
            fn size() -> usize {{
                {structure_size}
            }}

            fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {{
                let mut _pos = at;
                let mut output = Self::default();
                {read_in}
                Ok(output)
            }}

            fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {{
                let mut _pos = at;
                {write_out}
                Ok(())
            }}
        }}

        impl DynamicTagData for {struct_name} {{
            fn get_field(&self, field: &str) -> Option<&dyn DynamicTagData> {{
                match field {{
                    {getter}
                    _ => None
                }}
            }}

            fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn DynamicTagData> {{
                match field {{
                    {getter_mut}
                    _ => None
                }}
            }}

            fn fields(&self) -> &'static [&'static str] {{
                &[{field_list}]
            }}

            fn as_any(&self) -> &dyn Any {{
                self
            }}

            fn as_any_mut(&mut self) -> &mut dyn Any {{
                self
            }}

            fn data_type(&self) -> DynamicTagDataType {{
                DynamicTagDataType::Block
            }}
        }}").parse::<TokenStream>().unwrap();

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);

        tokens
    }
}

impl ToTokenStream for Enum {
    fn to_token_stream(&self, _definitions: &ParsedDefinitions) -> TokenStream {
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
        impl TagData for {struct_name} {{
            fn size() -> usize {{
                <u16 as TagDataSimplePrimitive>::size()
            }}

            fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {{
                let input = u16::read_from_tag_file(data, at, struct_end, extra_data_cursor)?;
                match input {{
                    {read_in}
                    _ => Err(Error::InvalidEnum)
                }}
            }}

            fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {{
                (*self as u16).write_to_tag_file(data, at, struct_end)
            }}
        }}

        impl DynamicTagData for {struct_name} {{
            fn get_field(&self, field: &str) -> Option<&dyn DynamicTagData> {{
                None
            }}

            fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn DynamicTagData> {{
                None
            }}

            fn fields(&self) -> &'static [&'static str] {{
                &[]
            }}

            fn as_any(&self) -> &dyn Any {{
                self
            }}

            fn as_any_mut(&mut self) -> &mut dyn Any {{
                self
            }}

            fn data_type(&self) -> DynamicTagDataType {{
                DynamicTagDataType::Enum
            }}
        }}

        ").parse::<TokenStream>().unwrap();

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);

        tokens
    }
}

impl ToTokenStream for Bitfield {
    fn to_token_stream(&self, _definitions: &ParsedDefinitions) -> TokenStream {
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

        // Generate readers/writers for converting between u<width> to the bitfield
        let width = self.width;
        let write_out = writeln_for_each_field!("output |= value.{field} as u{width} * {value};");
        let read_in = writeln_for_each_field!("{field}: (value & {value}) != 0,");
        let getter = writeln_for_each_field!("\"{field}\" => Some(&self.{field}), // {value}");
        let getter_mut = writeln_for_each_field!("\"{field}\" => Some(&mut self.{field}), // {value}");
        let list = writeln_for_each_field!("\"{field}\", // {value}");

        // Do not read/write cache_only stuff from tag files
        let cache_only_mask = self.fields.iter()
            .map(|f| match f.flags.cache_only { true => f.value, false => 0 } )
            .reduce(|a, b| a | b)
            .unwrap();
        let not_cache_only = !cache_only_mask;

        let functions = format!("
        impl From<u{width}> for {struct_name} {{
            fn from(value: u{width}) -> Self {{
                Self {{
                    {read_in}
                }}
            }}
        }}

        impl From<{struct_name}> for u{width} {{
            fn from(value: {struct_name}) -> Self {{
                let mut output: Self = 0;
                {write_out}
                output
            }}
        }}

        impl DynamicTagData for {struct_name} {{
            fn get_field(&self, field: &str) -> Option<&dyn DynamicTagData> {{
                match field {{
                    {getter}
                    _ => None
                }}
            }}

            fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn DynamicTagData> {{
                match field {{
                    {getter_mut}
                    _ => None
                }}
            }}

            fn fields(&self) -> &'static [&'static str] {{
                &[
                    {list}
                ]
            }}

            fn as_any(&self) -> &dyn Any {{
                self
            }}

            fn as_any_mut(&mut self) -> &mut dyn Any {{
                self
            }}

            fn data_type(&self) -> DynamicTagDataType {{
                DynamicTagDataType::Block
            }}
        }}

        impl TagData for {struct_name} {{
            fn size() -> usize {{
                <u{width} as TagData>::size()
            }}

            fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {{
                let read_in = u{width}::read_from_tag_file(data, at, struct_end, extra_data_cursor)? & {not_cache_only};
                Ok(read_in.into())
            }}

            fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {{
                let write_out: u{width} = (*self).into();
                (write_out & {not_cache_only}).write_to_tag_file(data, at, struct_end)
            }}
        }}").parse::<TokenStream>().unwrap();

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);

        tokens
    }
}

impl ToTokenStream for TagGroup {
    fn to_token_stream(&self, _definitions: &ParsedDefinitions) -> TokenStream {
        let struct_name = &self.struct_name;
        let version = self.version;
        let group = camel_case(&self.name);

        format!("impl PrimaryTagStruct for {struct_name} {{
            fn group() -> TagGroup {{
                TagGroup::{group}
            }}
            fn version() -> u16 {{
                {version}
            }}
        }}").parse().unwrap()
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

    let prefixes = &["Gbxm", "Ui", "Bsp", "Hud"];

    for p in prefixes {
        if result.contains(p) {
            result = result.replace(p, &p.to_ascii_uppercase());
        }
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
