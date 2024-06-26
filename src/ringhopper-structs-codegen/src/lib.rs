extern crate ringhopper_definitions;

use std::fmt::Write;
use std::borrow::Cow;

use ringhopper_definitions::{load_all_definitions, SizeableObject, Struct, NamedObject, Enum, Bitfield, StructFieldType, ObjectType, ParsedDefinitions, FieldCount, TagGroup, StaticValue, Flags};

use proc_macro::TokenStream;
use std::collections::HashSet;

#[proc_macro]
pub fn generate_ringhopper_structs(_: TokenStream) -> TokenStream {
    let definitions = load_all_definitions();
    let mut stream = TokenStream::default();

    for (_, obj) in &definitions.objects {
        stream.extend(obj.to_token_stream(&definitions));
    }

    let mut read_any_tag_lines = String::new();
    let mut read_any_map_lines = String::new();
    let mut referenceable_tag_groups_hint = String::new();
    let mut supported_groups_for_engines = String::new();
    let mut defaultable_tag_groups_hint = String::new();

    for (group_name, group) in &definitions.groups {
        let struct_name = &group.struct_name;
        let group_name_fixed = camel_case(&group_name);
        stream.extend(group.to_token_stream(&definitions));

        let mut groups: HashSet<String> = HashSet::new();
        let mut has_defaults = false;

        recursively_access_all_objects_in_definition(&definitions, &group.struct_name, |s| {
            for f in &s.fields {
                if let StructFieldType::Object(ObjectType::TagReference(r)) = &f.field_type {
                    for g in &r.allowed_groups {
                        groups.insert(format!("TagGroup::{}", camel_case(g)));
                    }
                }

                has_defaults |= f.default_value.is_some();
            }
        });

        if has_defaults {
            writeln!(&mut defaultable_tag_groups_hint, "TagGroup::{group_name_fixed} => true,").unwrap();
        }

        let mut list = String::new();
        for g in groups {
            list += &g;
            list += ",";
        }

        if let Some(e) = &group.supported_engines.supported_engines {
            let mut engines = String::new();
            for engine in resolve_all_engines_for_parents(&e, &definitions) {
                engines += "\"";
                engines += engine;
                engines += "\",";
            }
            writeln!(&mut supported_groups_for_engines, "TagGroup::{group_name_fixed} => (&[{engines}]).contains(&engine.name),").unwrap();
        }

        writeln!(referenceable_tag_groups_hint, "TagGroup::{group_name_fixed} => &[{list}],").unwrap();
        writeln!(read_any_tag_lines, "TagGroup::{group_name_fixed} => b(TagFile::read_tag_from_file_buffer::<{struct_name}>(file, ParseStrictness::Relaxed)),").unwrap();
        writeln!(read_any_map_lines, "TagGroup::{group_name_fixed} => b({struct_name}::read_from_map(map, tag_info.address, &tag_info.domain)),").unwrap();
    }

    stream.extend(format!("
    fn b<T: PrimaryTagStruct + Clone + 'static>(what: RinghopperResult<T>) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {{
        what.map(|b| Box::<T>::new(b) as Box<dyn PrimaryTagStructDyn>)
    }}

    /// Read the tag file buffer.
    ///
    /// Returns `Err` if the tag data is invalid, corrupt, or does not correspond to any known tag group.
    pub fn read_any_tag_from_file_buffer(file: &[u8], strictness: ParseStrictness) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {{
        let (header, _) = TagFile::load_header_and_data(file, strictness)?;

        match header.group {{
            {read_any_tag_lines}
            _ => Err(Error::TagGroupUnimplemented)
        }}
    }}

    /// Read the tag from the map.
    ///
    /// It is not recommended to call this directly, instead using extract_tag, as extract_tag will also fix any extraction
    /// artifacts, such as adding missing bitmap data.
    ///
    /// Returns `Err` if the tag data is invalid, corrupt, or does not correspond to any gettable tag group.
    pub fn read_any_tag_from_map<M: Map>(path: &TagPath, map: &M) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {{
        let tag_info = map.get_tag(path).ok_or_else(|| Error::TagNotFound(path.to_owned()))?;

        match path.group() {{
            {read_any_map_lines}
            _ => Err(Error::TagGroupUnimplemented)
        }}
    }}

    /// Get all tag groups this tag group can reference.
    pub fn get_all_referenceable_tag_groups_for_group(what: TagGroup) -> &'static [TagGroup] {{
        match what {{
            {referenceable_tag_groups_hint}
            _ => &[],
        }}
    }}

    /// Return `true` if the tag group uses defaults values.
    ///
    /// This is used to hint whether or not `TagDataDefaults::set_defaults` will do anything.
    pub fn group_has_default_in_definitions(what: TagGroup) -> bool {{
        match what {{
            {defaultable_tag_groups_hint}
            _ => false
        }}
    }}

    /// Return `true` if the tag group is supported on the target engine.
    pub fn group_supported_on_engine(group: TagGroup, engine: &Engine) -> bool {{
        match group {{
            {supported_groups_for_engines}
            _ => true
        }}
    }}
    ").parse::<TokenStream>());

    stream
}

fn recursively_access_all_objects_in_definition<P: FnMut(&Struct)>(definitions: &ParsedDefinitions, object: &str, mut predicate: P) {
    fn recursion<P: FnMut(&Struct)>(definitions: &ParsedDefinitions, object: &str, predicate: &mut P) {
        let object = &definitions.objects[object];
        if let NamedObject::Struct(s) = object {
            predicate(s);
            for f in &s.fields {
                if let StructFieldType::Object(n) = &f.field_type {
                    match n {
                        ObjectType::NamedObject(n) | ObjectType::Reflexive(n) => recursion(definitions, n, predicate),
                        _ => continue
                    }
                }
            }
        }
    }
    recursion(definitions, object, &mut predicate);
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

fn is_simple_bitfield(bitfield: &Bitfield) -> bool {
    for f in &bitfield.fields {
        if f.flags.cache_only || f.flags.non_cached {
            return false
        }
    }
    return true
}

fn is_simple_struct(structure: &Struct, parsed_definitions: &ParsedDefinitions) -> bool {
    for f in &structure.fields {
        if f.count != FieldCount::One {
            return false
        }

        if f.flags.cache_only || f.flags.non_cached {
            return false
        }

        match &f.field_type {
            StructFieldType::Object(o) => match o {
                ObjectType::NamedObject(o) => match &parsed_definitions.objects[o] {
                    NamedObject::Struct(s) => if !is_simple_struct(s, parsed_definitions) {
                        return false
                    },
                    NamedObject::Enum(_) => (),
                    NamedObject::Bitfield(b) => if !is_simple_bitfield(b) {
                        return false
                    }
                },
                ObjectType::Reflexive(_) => return false,
                ObjectType::TagReference(_) => return false,
                ObjectType::TagGroup => (),
                ObjectType::Data | ObjectType::FileData | ObjectType::BSPVertexData | ObjectType::UTF16String => return false,
                ObjectType::Float => (),
                ObjectType::U8 => (),
                ObjectType::U16 => (),
                ObjectType::U32 => (),
                ObjectType::I8 => (),
                ObjectType::I16 => (),
                ObjectType::I32 => (),
                ObjectType::TagID => (),
                ObjectType::ID => (),
                ObjectType::Index => (),
                ObjectType::Angle => (),
                ObjectType::Address => (),
                ObjectType::Vector2D => (),
                ObjectType::Vector3D => (),
                ObjectType::Vector2DInt => (),
                ObjectType::CompressedVector2D => (),
                ObjectType::CompressedVector3D => (),
                ObjectType::CompressedFloat => (),
                ObjectType::Plane2D => (),
                ObjectType::Plane3D => (),
                ObjectType::Euler2D => (),
                ObjectType::Euler3D => (),
                ObjectType::Rectangle => (),
                ObjectType::Quaternion => (),
                ObjectType::Matrix3x3 => (),
                ObjectType::ColorRGBFloat => (),
                ObjectType::ColorARGBFloat => (),
                ObjectType::ColorARGBInt => (),
                ObjectType::String32 => (),
                ObjectType::ScenarioScriptNodeValue => (),
            },
            StructFieldType::EditorSection(_) => (),
            StructFieldType::Padding(_) => ()
        }
    }

    true
}

fn recursive_look_for_defaults_for_struct(struct_name: &str, definitions: &ParsedDefinitions) -> bool {
    let s = if let NamedObject::Struct(s) = &definitions.objects[struct_name] {
        s
    }
    else {
        return false; // bitfields/enums maybe
    };
    for i in &s.fields {
        if i.default_value.is_some() {
            return true
        }
        match &i.field_type {
            StructFieldType::Object(o) => match o {
                ObjectType::NamedObject(n)
                | ObjectType::Reflexive(n) => if recursive_look_for_defaults_for_struct(n, definitions) {
                    return true
                },
                _ => ()
            },
            _ => ()
        }
    }
    false
}

impl ToTokenStream for Struct {
    fn to_token_stream(&self, definitions: &ParsedDefinitions) -> TokenStream {
        let struct_name = &self.name;
        let mut fields = String::new();

        let mut fields_with_types: Vec<String> = Vec::new();
        let mut fields_with_sizes: Vec<usize> = Vec::new();
        let mut fields_read_from_tags: Vec<bool> = Vec::new();
        let mut fields_read_from_caches: Vec<bool> = Vec::new();
        let mut reverse_field_matcher = String::new();

        let fields_with_names = self.fields.iter().map(|s| safe_str(&s.name, SafetyLevel::RustCompilation)).collect::<Vec<Cow<str>>>();
        let fields_with_matchers = self.fields.iter().map(|s| safe_str(&s.name, SafetyLevel::Matcher)).collect::<Vec<Cow<str>>>();

        let field_count = self.fields.len();
        let mut default_code = String::new();

        let mut main_group_struct = false;
        for g in &definitions.groups {
            if &g.1.struct_name == struct_name {
                writeln!(fields, "#[doc=\"This field is not part of the tag and is only used internally within Ringhopper 🐧\"] pub metadata: PrimaryTagStructMetadata,").unwrap();
                writeln!(default_code, "metadata: Default::default(),").unwrap();
                main_group_struct = true;
                break;
            }
        }

        // Can we write a simpler implementation?
        let simple_struct = !main_group_struct && is_simple_struct(&self, definitions);
        let clone = if simple_struct { "Copy, Clone" } else { "Clone" };

        for i in 0..field_count {
            let field_name = &fields_with_names[i];

            let field = &self.fields[i];
            let field_type = match &field.field_type {
                StructFieldType::Padding(n) => format!("Padding<[u8; {n}]>"),
                StructFieldType::EditorSection(_) => String::new(),
                StructFieldType::Object(o) => match o {
                    ObjectType::Angle => "Angle".to_owned(),
                    ObjectType::ColorARGBFloat => "ColorARGBFloat".to_owned(),
                    ObjectType::ColorRGBFloat => "ColorRGBFloat".to_owned(),
                    ObjectType::ColorARGBInt => "ColorARGBInt".to_owned(),
                    ObjectType::Data => "Data".to_owned(),
                    ObjectType::FileData => "FileData".to_owned(),
                    ObjectType::BSPVertexData => "BSPVertexData".to_owned(),
                    ObjectType::UTF16String => "UTF16String".to_owned(),
                    ObjectType::Euler2D => "Euler2D".to_owned(),
                    ObjectType::Euler3D => "Euler3D".to_owned(),
                    ObjectType::Float => "f64".to_owned(),
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
                    ObjectType::ID => "ID".to_owned(),
                    ObjectType::TagReference(_) => "TagReference".to_owned(),
                    ObjectType::TagGroup => "TagGroup".to_owned(),
                    ObjectType::U16 => "u16".to_owned(),
                    ObjectType::U32 => "u32".to_owned(),
                    ObjectType::U8 => "u8".to_owned(),
                    ObjectType::Vector2D => "Vector2D".to_owned(),
                    ObjectType::Vector3D => "Vector3D".to_owned(),
                    ObjectType::CompressedVector2D => "CompressedVector2D".to_owned(),
                    ObjectType::CompressedVector3D => "CompressedVector3D".to_owned(),
                    ObjectType::CompressedFloat => "CompressedFloat".to_owned(),
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

            match &field.field_type {
                StructFieldType::Object(o) => {
                    if !field.flags.exclude {
                        writeln!(&mut reverse_field_matcher, "if field == &self.{field_name} as *const dyn DynamicTagData {{ return \"{field_name}\" }}").unwrap();

                        let mut doc = String::new();
                        if let Some(n) = &field.flags.comment {
                            writeln!(&mut doc, "{n}\n\n").unwrap();
                        }
                        if let ObjectType::TagReference(t) = &o {
                            writeln!(&mut doc, "## Allowed groups").unwrap();
                            for g in &t.allowed_groups {
                                writeln!(&mut doc, "* [{g}](TagGroup::{reference}) ([struct info]({struct_ref}))", reference=camel_case(&g), struct_ref=camel_case(&g)).unwrap();
                            }
                            writeln!(&mut doc, "\n\n").unwrap();
                        }
                        if let Some(n) = &field.flags.developer_note {
                            writeln!(&mut doc, "## Developer note\n\n{n}").unwrap();
                        }
                        if field.flags.non_null {
                            writeln!(&mut doc, "## Non-null\n\nThis field **must** be set for the tag to be valid.\n\n").unwrap();
                        }
                        if field.flags.non_cached {
                            writeln!(&mut doc, "## Non-cached\n\nThis field is **only** present in tag files, not cache files.\n\n").unwrap();
                        }
                        if field.flags.cache_only {
                            writeln!(&mut doc, "## Cache only\n\nThis field is **only** present in cache files, not tag files.\n\n").unwrap();
                        }
                        if !doc.is_empty() {
                            doc = doc.replace("\\", "\\\\").replace("\"", "\\\"");
                            writeln!(&mut fields, "#[doc=\"{doc}\"]").unwrap();
                        }
                        writeln!(&mut fields, "pub {field_name}: {field_type},").unwrap();
                        if let ObjectType::TagReference(reference) = &o {
                            writeln!(&mut default_code, "{field_name}: TagReference::Null(TagGroup::{}),", camel_case(&reference.allowed_groups[0])).unwrap();
                        }
                        else {
                            writeln!(&mut default_code, "{field_name}: Default::default(),").unwrap();
                        }
                    }
                    fields_read_from_tags.push(!field.flags.cache_only && !field.flags.exclude);
                    fields_read_from_caches.push(!field.flags.non_cached && !field.flags.exclude);
                },
                _ => {
                    fields_read_from_tags.push(false);
                    fields_read_from_caches.push(false);
                }
            }
            fields_with_types.push(field_type);
            fields_with_sizes.push(field.size(definitions));
        }

        let structure = format!("
        #[derive({clone}, PartialEq, Debug)]
        pub struct {struct_name} {{
            {fields}
        }}").parse::<TokenStream>().unwrap();

        let structure_size = self.size;

        let mut write_out = String::new();
        let mut read_tag_in = String::new();
        let mut read_map_in = String::new();

        let mut field_list = String::new();
        let mut getter = String::new();
        let mut getter_mut = String::new();
        let mut metadata_matcher = String::new();

        for i in 0..field_count {
            let field = &self.fields[i];

            if field.flags.exclude || !matches!(field.field_type, StructFieldType::Object(_)) {
                continue;
            }

            let allowed_references = if let StructFieldType::Object(ObjectType::TagReference(reference)) = &field.field_type {
                let mut list = String::new();
                for g in &reference.allowed_groups {
                    list += &format!("TagGroup::{},", camel_case(g));
                }
                format!("Some(&[{list}])")
            }
            else {
                "None".to_owned()
            };

            let metadata = build_metadata(&field.flags, &allowed_references);
            let field_name = &fields_with_names[i];
            let field_matcher = &fields_with_matchers[i];
            writeln!(&mut metadata_matcher, "\"{field_matcher}\" => Some({metadata}),").unwrap();
            write!(&mut field_list, "\"{field_matcher}\",").unwrap();
            writeln!(&mut getter, "\"{field_matcher}\" => Some(&self.{field_name}),").unwrap();
            writeln!(&mut getter_mut, "\"{field_matcher}\" => Some(&mut self.{field_name}),").unwrap();
        }

        // Tag I/O
        for i in 0..field_count {
            let length = &fields_with_sizes[i];

            let field_name = &fields_with_names[i];
            let field_type = &fields_with_types[i];

            let little_endian = self.fields[i].flags.little_endian_in_tags;
            let read_tag_code = if little_endian {
                format!("<{field_type}>::read::<LittleEndian>(data, _pos, struct_end)?")
            }
            else if simple_struct {
                format!("<{field_type}>::read::<B>(data, _pos, struct_end)?")
            }
            else {
                format!("<{field_type}>::read_from_tag_file(data, _pos, struct_end, extra_data_cursor)?")
            };

            if fields_read_from_tags[i] {
                writeln!(&mut read_tag_in, "output.{field_name} = {read_tag_code};").unwrap();
                if little_endian {
                    writeln!(&mut write_out, "self.{field_name}.write::<LittleEndian>(data, _pos, struct_end)?;").unwrap();
                }
                else if simple_struct {
                    writeln!(&mut write_out, "self.{field_name}.write::<B>(data, _pos, struct_end)?;").unwrap();
                }
                else {
                    writeln!(&mut write_out, "self.{field_name}.write_to_tag_file(data, _pos, struct_end)?;").unwrap();
                }
            }
            else if let StructFieldType::Object(object_type) = &self.fields[i].field_type {
                let should_output_code_anyway = match object_type {
                    ObjectType::NamedObject(o) => match definitions.objects[o] {
                        NamedObject::Enum(_) | NamedObject::Bitfield(_) => false,
                        NamedObject::Struct(_) => true
                    },
                    ObjectType::Reflexive(_) => true,
                    ObjectType::Data => true,
                    ObjectType::TagReference(_) => true,
                    _ => false
                };
                if should_output_code_anyway {
                    writeln!(&mut read_tag_in, "{read_tag_code};").unwrap();
                    match &self.fields[i].field_type {
                        StructFieldType::Object(ObjectType::TagReference(t)) => {
                            let best_group = camel_case(&t.allowed_groups[0]);
                            writeln!(&mut write_out, "TagReference::Null(TagGroup::{best_group}).write_to_tag_file(data, _pos, struct_end)?;").unwrap()
                        }
                        _ => writeln!(&mut write_out, "<{field_type}>::default().write_to_tag_file(data, _pos, struct_end)?;").unwrap()
                    }
                }
            }
            writeln!(&mut write_out, "let _pos = _pos.add_overflow_checked({length})?;").unwrap();
            writeln!(&mut read_tag_in, "let _pos = _pos.add_overflow_checked({length})?;").unwrap();
        }

        // Map I/O
        if !simple_struct {
            for i in 0..field_count {
                let length = &fields_with_sizes[i];
                if fields_read_from_caches[i] {
                    let field_name = &fields_with_names[i];
                    let field_type = &fields_with_types[i];

                    let read_map_code = if self.flags.shifted_by_one {
                        "(u16::read_from_map(map, _pos, domain_type)?.wrapping_add(1)).try_into()?".to_owned()
                    }
                    else if field_type == "BSPVertexData" {
                        let compressed = field_name.starts_with("compressed");
                        format!("<{field_type}>::read_from_map_with_offset(map, _pos, domain_type, {compressed}, output.rendered_vertices.vertex_count as usize, output.rendered_vertices.offset as usize, output.lightmap_vertices.vertex_count as usize, output.lightmap_vertices.offset as usize)?")
                    }
                    else {
                        format!("<{field_type}>::read_from_map(map, _pos, domain_type)?")
                    };
                    writeln!(&mut read_map_in, "output.{field_name} = {read_map_code};").unwrap();
                }
                writeln!(&mut read_map_in, "let _pos = _pos.add_overflow_checked({length})?;").unwrap();
            }
        }
        else {
            read_map_in = "BAD".to_owned();
        }

        // Defaulting code
        let zero_defaulting_code = if recursive_look_for_defaults_for_struct(struct_name, definitions) {
            let mut all_defaulting_code = String::new();
            let mut all_undefaulting_code = String::new();

            for i in 0..field_count {
                let field = &self.fields[i];
                let field_name = &fields_with_names[i];
                let field_type = &fields_with_types[i].replace("<", "::<");

                if let Some(n) = &field.default_value {
                    let merge_vector = |vector: &[StaticValue]| -> String {
                        if vector.len() == 1 {
                            return if let StructFieldType::Object(ObjectType::Angle) = field.field_type {
                                format!("Angle::from_degrees({} as f32)", &vector[0])
                            }
                            else {
                                vector[0].to_string()
                            }
                        }

                        let mut q = "[".to_string();
                        for i in vector {
                            write!(&mut q, "{i},").unwrap()
                        }
                        write!(&mut q, "].into()").unwrap();
                        q
                    };

                    let default_value = match field.count {
                        FieldCount::One | FieldCount::Array(_) => merge_vector(n.as_slice()),
                        FieldCount::Bounds => {
                            let (from, to) = n.split_at(n.len() / 2);
                            format!("(Bounds {{ lower: {from}, upper: {to} }})", from=merge_vector(from), to=merge_vector(to))
                        }
                    };

                    writeln!(&mut all_defaulting_code, "if self.{field_name} == {field_type}::default() {{
                        self.{field_name} = {default_value};
                    }}").unwrap();
                    writeln!(&mut all_undefaulting_code, "if self.{field_name} == {default_value} {{
                        self.{field_name} = Default::default();
                    }}").unwrap();
                }
                else {
                    match &field.field_type {
                        StructFieldType::Object(ObjectType::NamedObject(o))
                        | StructFieldType::Object(ObjectType::Reflexive(o)) => {
                            if recursive_look_for_defaults_for_struct(o, definitions) {
                                writeln!(&mut all_defaulting_code, "self.{field_name}.set_defaults();").unwrap();
                                writeln!(&mut all_undefaulting_code, "self.{field_name}.unset_defaults();").unwrap();
                            }
                        },
                        _ => ()
                    }
                }
            }

            format!("impl TagDataDefaults for {struct_name} {{
                fn set_defaults(&mut self) {{
                    {all_defaulting_code}
                }}
                fn unset_defaults(&mut self) {{
                    {all_undefaulting_code}
                }}
            }}")
        }
        else {
            format!("impl TagDataDefaults for {struct_name} {{}}")
        };

        let tag_data_functions = if simple_struct {
            format!("impl SimpleTagData for {struct_name} {{
                fn simple_size() -> usize {{
                    {structure_size}
                }}

                fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {{
                    let _pos = at;
                    let mut output = Self::default();
                    {read_tag_in}
                    Ok(output)
                }}

                fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {{
                    let mut _pos = at;
                    {write_out}
                    Ok(())
                }}
            }}")
        }
        else {
            format!("impl TagData for {struct_name} {{
                fn size() -> usize {{
                    {structure_size}
                }}

                fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {{
                    let _pos = at;
                    let mut output = Self::default();
                    {read_tag_in}
                    Ok(output)
                }}

                fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {{
                    let mut _pos = at;
                    {write_out}
                    Ok(())
                }}

                fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {{
                    let _pos = address;
                    let mut output = Self::default();
                    {read_map_in}
                    Ok(output)
                }}
            }}")
        }.parse::<TokenStream>().unwrap();

        let aux_functions = format!("impl Default for {struct_name} {{
            fn default() -> Self {{
                Self {{
                    {default_code}
                }}
            }}
        }}

        {zero_defaulting_code}

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

            fn get_metadata_for_field(&self, field: &str) -> Option<TagFieldMetadata> {{
                match field {{
                    {metadata_matcher}
                    _ => None
                }}
            }}

            fn name_of_field_from_ptr(&self, field: *const dyn DynamicTagData) -> &'static str {{
                {reverse_field_matcher}
                panic!(\"field does not point to anything in this block\");
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
        tokens.extend(tag_data_functions);
        tokens.extend(aux_functions);

        tokens
    }
}

impl ToTokenStream for Enum {
    fn to_token_stream(&self, _definitions: &ParsedDefinitions) -> TokenStream {
        let field_names_rust = self.options.iter().map(|s| camel_case(&s.name)).collect::<Vec<String>>();
        let field_names_matchers = self.options.iter().map(|s| safe_str(&s.name, SafetyLevel::Matcher)).collect::<Vec<Cow<str>>>();

        macro_rules! writeln_for_each_field {
            ($($fmt:expr)*) => {{
                let mut out = String::new();
                for i in 0..self.options.len() {
                    let option = &self.options[i];
                    if option.flags.exclude {
                        continue
                    }
                    writeln!(&mut out, $($fmt)*, field_camel_case=field_names_rust[i], field_snake_case=field_names_matchers[i], value=option.value).unwrap();
                }
                out
            }};
        }

        let struct_name = &self.name;
        let fields = writeln_for_each_field!("{field_camel_case} = {value}, // {field_snake_case}");
        let read_in = writeln_for_each_field!("{value} => Ok(Self::{field_camel_case}), // {field_snake_case}");
        let field_name_list = writeln_for_each_field!("\"{field_snake_case}\", // {value}, {field_camel_case}");
        let str_to_enum = writeln_for_each_field!("\"{field_snake_case}\" => Some(Self::{field_camel_case}), // {value}");
        let enum_to_str = writeln_for_each_field!("Self::{field_camel_case} => \"{field_snake_case}\", // {value}");

        let structure = format!("
        #[derive(Copy, Clone, PartialEq, Default, Debug)]
        #[repr(u16)]
        pub enum {struct_name} {{
            #[default]
            {fields}
        }}").parse::<TokenStream>().unwrap();

        let functions = format!("

        impl TryFrom<u16> for {struct_name} {{
            type Error = Error;
            fn try_from(value: u16) -> Result<Self, Self::Error> {{
                match value {{
                    {read_in}
                    _ => Err(Error::InvalidEnum)
                }}
            }}
        }}

        impl SimpleTagData for {struct_name} {{
            fn simple_size() -> usize {{
                u16::simple_size()
            }}

            fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {{
                Ok(u16::read::<B>(data, at, struct_end)?.try_into().unwrap_or_else(|_| Default::default()))
            }}

            fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {{
                (*self as u16).write::<B>(data, at, struct_end)
            }}
        }}

        impl Display for {struct_name} {{
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {{
                f.write_str(self.to_str())
            }}
        }}

        impl DynamicTagData for {struct_name} {{
            fn get_field(&self, field: &str) -> Option<&dyn DynamicTagData> {{
                None
            }}

            fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn DynamicTagData> {{
                None
            }}

            fn get_metadata_for_field(&self, field: &str) -> Option<TagFieldMetadata> {{
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

            fn as_enum(&self) -> Option<&dyn DynamicEnum> {{
                Some(self)
            }}

            fn as_enum_mut(&mut self) -> Option<&mut dyn DynamicEnum> {{
                Some(self)
            }}
        }}

        impl TagDataDefaults for {struct_name} {{}}

        impl DynamicEnumImpl for {struct_name} {{
            fn from_str(value: &str) -> Option<Self> {{
                match value {{
                    {str_to_enum}
                    _ => None
                }}
            }}
            fn to_str(&self) -> &'static str {{
                match *self {{
                    {enum_to_str}
                }}
            }}
            fn str_vals() -> &'static [&'static str] {{
                &[
                    {field_name_list}
                ]
            }}
        }}").parse::<TokenStream>().unwrap();

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);

        tokens
    }
}

impl ToTokenStream for Bitfield {
    fn to_token_stream(&self, _definitions: &ParsedDefinitions) -> TokenStream {
        let struct_name = &self.name;

        let field_names_rust = self.fields.iter().map(|s| safe_str(&s.name, SafetyLevel::RustCompilation)).collect::<Vec<Cow<str>>>();
        let field_names_matchers = self.fields.iter().map(|s| safe_str(&s.name, SafetyLevel::Matcher)).collect::<Vec<Cow<str>>>();

        macro_rules! writeln_for_each_field {
            ($($fmt:expr)*) => {{
                let mut out = String::new();
                for i in 0..self.fields.len() {
                    let field = &self.fields[i];
                    if field.flags.exclude {
                        continue
                    }
                    writeln!(&mut out, $($fmt)*, field_rust=field_names_rust[i], field_matcher=field_names_matchers[i], value=field.value).unwrap();
                }
                out
            }};
        }

        let fields = writeln_for_each_field!("pub {field_rust}: bool, // {field_matcher}, {value}");
        let structure = format!("
        #[derive(Copy, Clone, PartialEq, Default, Debug)]
        pub struct {struct_name} {{
            {fields}
        }}").parse::<TokenStream>().unwrap();

        // Generate readers/writers for converting between u<width> to the bitfield
        let width = self.width;
        let write_out = writeln_for_each_field!("output |= value.{field_rust} as u{width} * {value}; // {field_matcher}");
        let read_in = writeln_for_each_field!("{field_rust}: (value & {value}) != 0, // {field_matcher}");
        let getter = writeln_for_each_field!("\"{field_matcher}\" => Some(&self.{field_rust}), // {value}");
        let getter_mut = writeln_for_each_field!("\"{field_matcher}\" => Some(&mut self.{field_rust}), // {value}");
        let list = writeln_for_each_field!("\"{field_matcher}\", // {field_rust}, {value}");

        let mut all_metadata = String::new();
        for i in 0..self.fields.len() {
            let field = &self.fields[i];
            if field.flags.exclude {
                continue
            }
            let field_matcher = &field_names_matchers[i];
            let metadata = build_metadata(&field.flags, "None");
            writeln!(&mut all_metadata, "\"{field_matcher}\" => Some({metadata}),").unwrap();
        }

        // Do not read/write cache_only stuff from tag files
        let cache_only_mask = self.fields.iter()
            .map(|f| match f.flags.cache_only { true => f.value, false => 0 } )
            .reduce(|a, b| a | b)
            .unwrap();
        let not_cache_only = !cache_only_mask;

        let tag_only_mask = self.fields.iter()
            .map(|f| match f.flags.non_cached { true => f.value, false => 0 } )
            .reduce(|a, b| a | b)
            .unwrap();
        let not_tag_only = !tag_only_mask;

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

        impl TagDataDefaults for {struct_name} {{}}

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

            fn get_metadata_for_field(&self, field: &str) -> Option<TagFieldMetadata> {{
                match field {{
                    {all_metadata}
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
        }}").parse::<TokenStream>().unwrap();

        let parse_functions = if is_simple_bitfield(self) {
            format!("impl SimpleTagData for {struct_name} {{
                fn simple_size() -> usize {{
                    u{width}::simple_size()
                }}

                fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {{
                    let read_in = u{width}::read::<B>(data, at, struct_end)?;
                    Ok(read_in.into())
                }}

                fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {{
                    let write_out: u{width} = (*self).into();
                    write_out.write::<B>(data, at, struct_end)
                }}
            }}").parse::<TokenStream>().unwrap()
        }
        else {
            format!("impl TagData for {struct_name} {{
                fn size() -> usize {{
                    u{width}::simple_size()
                }}

                fn read_from_tag_file(data: &[u8], at: usize, struct_end: usize, extra_data_cursor: &mut usize) -> RinghopperResult<Self> {{
                    let read_in = u{width}::read_from_tag_file(data, at, struct_end, extra_data_cursor)? & {not_cache_only};
                    Ok(read_in.into())
                }}

                fn write_to_tag_file(&self, data: &mut Vec<u8>, at: usize, struct_end: usize) -> RinghopperResult<()> {{
                    let output = u{width}::from(*self) & {not_cache_only};
                    output.write_to_tag_file(data, at, struct_end)
                }}

                fn read_from_map<M: Map>(map: &M, address: usize, domain_type: &DomainType) -> RinghopperResult<Self> {{
                    let read_in = u{width}::read_from_map(map, address, domain_type)? & {not_tag_only};
                    Ok(read_in.into())
                }}
            }}").parse::<TokenStream>().unwrap()
        };

        let mut tokens = TokenStream::default();

        tokens.extend(structure);
        tokens.extend(functions);
        tokens.extend(parse_functions);

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
            fn metadata(&self) -> &PrimaryTagStructMetadata {{
                &self.metadata
            }}
            fn metadata_mut(&mut self) -> &mut PrimaryTagStructMetadata {{
                &mut self.metadata
            }}
        }}").parse().unwrap()
    }
}

fn camel_case(string: &str) -> String {
    let safe = safe_str(string, SafetyLevel::RustCompilation);

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

    let prefixes = &["Gbxm", "Ui", "Bsp", "Hud", "Dxt", "Pcm", "Bc7", "Adpcm", "A1r5g5b5", "R5g6b5", "A4r4g4b4", "A8y8", "Ay8", "A8r8g8b8", "X8r8g8b8", "Ucs"];

    for p in prefixes {
        if result.contains(p) {
            result = result.replace(p, &p.to_ascii_uppercase());
        }
    }

    result
}

#[derive(Copy, Clone, PartialEq)]
enum SafetyLevel {
    Matcher,
    RustCompilation
}

fn resolve_all_engines_for_parents<'a>(parents: &'a [String], definitions: &'a ParsedDefinitions) -> HashSet<&'a str> {
    let mut result = HashSet::new();

    for i in parents {
        result.insert(i.as_str());
    }

    loop {
        let mut added_something = false;
        for &i in &result.clone() {
            for (engine_name, engine) in &definitions.engines {
                let engine_name = engine_name.as_str();
                if engine.inherits.as_ref().is_some_and(|inherits| inherits == i) {
                    if !result.contains(engine_name) {
                        result.insert(engine_name);
                        added_something = true;
                    }
                }
            }
        }
        if !added_something {
            break;
        }
    }

    result
}

fn safe_str(string: &str, safety_level: SafetyLevel) -> Cow<str> {
    let mut string = Cow::Borrowed(string);

    if string.is_empty() {
        return string
    }

    let underscored_characters = ['-', ' '];
    if string.chars().any(|c| underscored_characters.contains(&c)) {
        string = Cow::Owned(string.chars().map(|c| if underscored_characters.contains(&c) { '_' } else { c }).collect())
    }

    let forbidden_chars = ['\'', '"', '(', ')'];
    if string.chars().any(|c| forbidden_chars.contains(&c)) {
        string = Cow::Owned(string.chars().filter(|c| !forbidden_chars.contains(&c)).collect())
    }

    if string.chars().any(|c| c.is_ascii_uppercase()) {
        string = Cow::Owned(string.to_ascii_lowercase());
    }

    if safety_level == SafetyLevel::RustCompilation {
        if string.chars().next().unwrap().is_numeric() {
            string = Cow::Owned(format!("_{string}"));
        }

        if string == "type" {
            string = Cow::Borrowed("_type");
        }

        if string == "loop" {
            string = Cow::Borrowed("_loop");
        }
    }

    string
}

fn build_metadata(flags: &Flags, allowed_references: &str) -> String {
    let comment = if let Some(n) = &flags.comment {
        let r = n.replace("\\", "\\\\").replace("\"", "\\\"").replace("\n", "\\n");
        format!("Some(\"{r}\")")
    }
    else {
        "None".to_owned()
    };
    let read_only = flags.uneditable_in_editor;
    let cache_only = flags.cache_only;
    let non_cached = flags.non_cached;

    format!("TagFieldMetadata {{
        comment: {comment},
        read_only: {read_only},
        cache_only: {cache_only},
        non_cached: {non_cached},
        allowed_references: {allowed_references}
    }}")
}