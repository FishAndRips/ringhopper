use std::collections::VecDeque;

use super::*;
use serde_json::*;

macro_rules! oget_name {
    ($obj:expr) => {
        $obj.get("name").map(|c| c.as_str().unwrap()).unwrap_or("<noname>")
    };
}

macro_rules! oget {
    ($obj:expr, $field:expr) => {
        $obj.get($field).unwrap_or_else(|| panic!("no such field {name}::{field}", field=$field, name=oget_name!($obj)))
    };
}

macro_rules! oget_str {
    ($obj:expr, $field:expr) => {
        oget!($obj, $field).as_str().unwrap_or_else(|| panic!("expected {name}::{field} to be a string", field=$field, name=oget_name!($obj)))
    };
}

macro_rules! oget_number {
    ($obj:expr, $field:expr, $accessor:tt) => {
        oget!($obj, $field)
            .as_number()
            .unwrap_or_else(|| panic!("expected {name}::{field} to be a number", field=$field, name=oget_name!($obj)))
            .$accessor()
            .unwrap_or_else(|| panic!("expected {name}::{field} to be a certain type of number", field=$field, name=oget_name!($obj)))
    };
}

macro_rules! oget_size {
    ($obj:expr) => {
        oget_number!($obj, "size", as_u64) as usize
    };
}

/// Recursively resolve parent groups (e.g. object -> [unit, device, etc.] -> [biped, vehicle, device_machine, etc.])
fn get_all_child_groups(parent: &String, groups: &HashMap<String, TagGroup>) -> Vec<String> {
    if parent == "*" {
        return groups.keys().map(|f| f.to_owned()).collect()
    }

    let mut result = Vec::new();
    for (group_name, group) in groups {
        if group.supergroup.as_ref() == Some(parent) {
            result.push(group_name.to_owned());
            result.append(&mut get_all_child_groups(group_name, groups));
        }
    }
    result
}

impl ParsedTagData {
    fn load_from_json(&mut self, objects: &Vec<Map<String, Value>>) {
        for object in objects {
            let object_type = oget_str!(object, "type");
            let object_name = oget_str!(object, "name").to_owned();

            if object_type == "group" {
                let parent_maybe = object.get("supergroup").map(|g| g.as_str().unwrap().to_owned());

                assert!(!self.groups.contains_key(&object_name), "duplicate group {object_name} detected");
                self.groups.insert(object_name.clone(), TagGroup {
                    name: object_name,
                    struct_name: oget_str!(object, "struct").to_owned(),
                    supergroup: parent_maybe
                });

                continue
            }

            // Now add the object
            assert!(!self.objects.contains_key(&object_name), "duplicate object {object_name} detected");
            self.objects.insert(object_name, NamedObject::load_from_json(object));
        }
    }

    // Fix all tag references to have child groups
    fn resolve_parent_class_references(&mut self) {
        for (_, named_object) in &mut self.objects {
            if let NamedObject::Struct(s) = named_object {
                for f in &mut s.fields {
                    if let StructFieldType::Object(ObjectType::TagReference(r)) = &mut f.field_type {
                        let mut new_fields: Option<Vec<String>> = None;

                        for group in &r.allowed_groups {
                            let mut children = get_all_child_groups(group, &self.groups);
                            if children.is_empty() {
                                continue
                            }
                            if new_fields.is_none() {
                                new_fields = Some(r.allowed_groups.clone());
                            }
                            new_fields.as_mut().unwrap().append(&mut children);
                        }

                        if let Some(f) = new_fields {
                            r.allowed_groups = f;
                        }

                        r.allowed_groups.retain(|f| f != "*");
                        r.allowed_groups.dedup();
                    }
                }
            }
        }
    }

    fn assert_valid(&self) {
        for (group_name, group) in &self.groups {
            let group_name_in_struct = &group.name;
            assert_eq!(group_name_in_struct, group_name, "group name `{group_name_in_struct}` not consistent with name `{group_name}` in map");

            let struct_name = &group.struct_name;
            self.objects.get(struct_name).unwrap_or_else(|| panic!("group {group_name} refers to struct {struct_name} which does not exist"));

            if let Some(s) = &group.supergroup {
                self.groups.get(s).unwrap_or_else(|| panic!("group {group_name}'s supergroup refers to group {s} which does not exist"));
            }
        }

        for (object_name, object) in &self.objects {
            let name_in_object = object.name();
            assert_eq!(name_in_object, object_name, "object name `{name_in_object}` not consistent with name `{object_name}` in map");

            match object {
                NamedObject::Bitfield(b) => assert!(b.fields.len() <= b.width as usize, "bitfield {object_name} has too many fields; {} / {}", b.fields.len(), b.width),
                NamedObject::Enum(e) => assert!(e.options.len() <= u16::MAX as usize, "enum {object_name} has too many options, {} / {}", e.options.len(), u16::MAX),
                NamedObject::Struct(s) => {
                    for f in &s.fields {
                        let field_name = &f.name;
                        match &f.field_type {
                            StructFieldType::Object(ObjectType::NamedObject(o)) => {
                                self.objects.get(o).unwrap_or_else(|| panic!("{object_name}::{field_name} type refers to object {o} which does not exist"));
                            },
                            StructFieldType::Object(ObjectType::TagReference(r)) => {
                                for g in &r.allowed_groups {
                                    if g == "*" {
                                        continue
                                    }
                                    self.groups.get(g).unwrap_or_else(|| panic!("{object_name}::{field_name} reference refers to tag group {g} which does not exist"));
                                }
                            },
                            StructFieldType::Object(ObjectType::Reflexive(r)) => {
                                self.objects.get(r).unwrap_or_else(|| panic!("{object_name}::{field_name} reflexive refers to object {r} which does not exist"));
                            },
                            _ => ()
                        }
                    }

                    s.assert_size_is_correct(self);
                }
            }
        }
    }
}

fn get_all_definitions() -> Vec<Map<String, Value>> {
    let mut jsons: HashMap<&'static str, &'static [u8]> = HashMap::new();

    jsons.insert("actor_variant", include_bytes!("../../json/actor_variant.json"));
    jsons.insert("actor", include_bytes!("../../json/actor.json"));
    jsons.insert("antenna", include_bytes!("../../json/antenna.json"));
    jsons.insert("biped", include_bytes!("../../json/biped.json"));
    jsons.insert("bitfield", include_bytes!("../../json/bitfield.json"));
    jsons.insert("bitmap", include_bytes!("../../json/bitmap.json"));
    jsons.insert("camera_track", include_bytes!("../../json/camera_track.json"));
    jsons.insert("color_table", include_bytes!("../../json/color_table.json"));
    jsons.insert("continuous_damage_effect", include_bytes!("../../json/continuous_damage_effect.json"));
    jsons.insert("contrail", include_bytes!("../../json/contrail.json"));
    jsons.insert("damage_effect", include_bytes!("../../json/damage_effect.json"));
    jsons.insert("decal", include_bytes!("../../json/decal.json"));
    jsons.insert("detail_object_collection", include_bytes!("../../json/detail_object_collection.json"));
    jsons.insert("device_control", include_bytes!("../../json/device_control.json"));
    jsons.insert("device_light_fixture", include_bytes!("../../json/device_light_fixture.json"));
    jsons.insert("device_machine", include_bytes!("../../json/device_machine.json"));
    jsons.insert("device", include_bytes!("../../json/device.json"));
    jsons.insert("dialogue", include_bytes!("../../json/dialogue.json"));
    jsons.insert("effect", include_bytes!("../../json/effect.json"));
    jsons.insert("enum", include_bytes!("../../json/enum.json"));
    jsons.insert("equipment", include_bytes!("../../json/equipment.json"));
    jsons.insert("flag", include_bytes!("../../json/flag.json"));
    jsons.insert("fog", include_bytes!("../../json/fog.json"));
    jsons.insert("font", include_bytes!("../../json/font.json"));
    jsons.insert("garbage", include_bytes!("../../json/garbage.json"));
    jsons.insert("gbxmodel", include_bytes!("../../json/gbxmodel.json"));
    jsons.insert("globals", include_bytes!("../../json/globals.json"));
    jsons.insert("glow", include_bytes!("../../json/glow.json"));
    jsons.insert("grenade_hud_interface", include_bytes!("../../json/grenade_hud_interface.json"));
    jsons.insert("hud_globals", include_bytes!("../../json/hud_globals.json"));
    jsons.insert("hud_interface_types", include_bytes!("../../json/hud_interface_types.json"));
    jsons.insert("hud_message_text", include_bytes!("../../json/hud_message_text.json"));
    jsons.insert("hud_number", include_bytes!("../../json/hud_number.json"));
    jsons.insert("input_device_defaults", include_bytes!("../../json/input_device_defaults.json"));
    jsons.insert("item_collection", include_bytes!("../../json/item_collection.json"));
    jsons.insert("item", include_bytes!("../../json/item.json"));
    jsons.insert("lens_flare", include_bytes!("../../json/lens_flare.json"));
    jsons.insert("light_volume", include_bytes!("../../json/light_volume.json"));
    jsons.insert("light", include_bytes!("../../json/light.json"));
    jsons.insert("lightning", include_bytes!("../../json/lightning.json"));
    jsons.insert("material_effects", include_bytes!("../../json/material_effects.json"));
    jsons.insert("meter", include_bytes!("../../json/meter.json"));
    jsons.insert("model_animations", include_bytes!("../../json/model_animations.json"));
    jsons.insert("model_collision_geometry", include_bytes!("../../json/model_collision_geometry.json"));
    jsons.insert("model", include_bytes!("../../json/model.json"));
    jsons.insert("multiplayer_scenario_description", include_bytes!("../../json/multiplayer_scenario_description.json"));
    jsons.insert("object", include_bytes!("../../json/object.json"));
    jsons.insert("particle_system", include_bytes!("../../json/particle_system.json"));
    jsons.insert("particle", include_bytes!("../../json/particle.json"));
    jsons.insert("physics", include_bytes!("../../json/physics.json"));
    jsons.insert("placeholder", include_bytes!("../../json/placeholder.json"));
    jsons.insert("point_physics", include_bytes!("../../json/point_physics.json"));
    jsons.insert("preferences_network_game", include_bytes!("../../json/preferences_network_game.json"));
    jsons.insert("projectile", include_bytes!("../../json/projectile.json"));
    jsons.insert("scenario_structure_bsp", include_bytes!("../../json/scenario_structure_bsp.json"));
    jsons.insert("scenario", include_bytes!("../../json/scenario.json"));
    jsons.insert("scenery", include_bytes!("../../json/scenery.json"));
    jsons.insert("shader_environment", include_bytes!("../../json/shader_environment.json"));
    jsons.insert("shader_model", include_bytes!("../../json/shader_model.json"));
    jsons.insert("shader_transparent_chicago_extended", include_bytes!("../../json/shader_transparent_chicago_extended.json"));
    jsons.insert("shader_transparent_chicago", include_bytes!("../../json/shader_transparent_chicago.json"));
    jsons.insert("shader_transparent_generic", include_bytes!("../../json/shader_transparent_generic.json"));
    jsons.insert("shader_transparent_glass", include_bytes!("../../json/shader_transparent_glass.json"));
    jsons.insert("shader_transparent_meter", include_bytes!("../../json/shader_transparent_meter.json"));
    jsons.insert("shader_transparent_plasma", include_bytes!("../../json/shader_transparent_plasma.json"));
    jsons.insert("shader_transparent_water", include_bytes!("../../json/shader_transparent_water.json"));
    jsons.insert("shader", include_bytes!("../../json/shader.json"));
    jsons.insert("sky", include_bytes!("../../json/sky.json"));
    jsons.insert("sound_environment", include_bytes!("../../json/sound_environment.json"));
    jsons.insert("sound_looping", include_bytes!("../../json/sound_looping.json"));
    jsons.insert("sound_scenery", include_bytes!("../../json/sound_scenery.json"));
    jsons.insert("sound", include_bytes!("../../json/sound.json"));
    jsons.insert("string_list", include_bytes!("../../json/string_list.json"));
    jsons.insert("tag_collection", include_bytes!("../../json/tag_collection.json"));
    jsons.insert("ui_widget_collection", include_bytes!("../../json/ui_widget_collection.json"));
    jsons.insert("ui_widget_definition", include_bytes!("../../json/ui_widget_definition.json"));
    jsons.insert("unicode_string_list", include_bytes!("../../json/unicode_string_list.json"));
    jsons.insert("unit_hud_interface", include_bytes!("../../json/unit_hud_interface.json"));
    jsons.insert("unit", include_bytes!("../../json/unit.json"));
    jsons.insert("vehicle", include_bytes!("../../json/vehicle.json"));
    jsons.insert("virtual_keyboard", include_bytes!("../../json/virtual_keyboard.json"));
    jsons.insert("weapon_hud_interface", include_bytes!("../../json/weapon_hud_interface.json"));
    jsons.insert("weapon", include_bytes!("../../json/weapon.json"));
    jsons.insert("weather_particle_system", include_bytes!("../../json/weather_particle_system.json"));
    jsons.insert("wind", include_bytes!("../../json/wind.json"));

    jsons.into_iter()
            .map(|(file,v)| (file, serde_json::from_slice::<Value>(v).unwrap_or_else(|e| panic!("failed to parse {file}: {e}"))))
            .map(|(file, v)| (file, v.as_array().map(|a| a.to_owned()).unwrap_or_else(|| panic!("failed to convert {file} to an array"))))
            .map(|(file, v)| {
                v.iter()
                    .map(|o| o.as_object().unwrap_or_else(|| panic!("invalid objects in {file}")).to_owned())
                    .collect::<Vec<Map<String, Value>>>()
            })
            .flatten()
            .collect()
}

#[test]
fn test_def() {
    let values = get_all_definitions();
    let mut parsed = ParsedTagData::default();
    parsed.load_from_json(&values);
    parsed.assert_valid();
    parsed.resolve_parent_class_references();
}

trait LoadFromSerdeJSON {
    fn load_from_json(object: &Map<String, Value>) -> Self;
}

impl LoadFromSerdeJSON for NamedObject {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let object_type = oget_str!(object, "type");
        match object_type {
            "struct" => Self::Struct(Struct::load_from_json(object)),
            "enum" => Self::Enum(Enum::load_from_json(object)),
            "bitfield" => Self::Bitfield(Bitfield::load_from_json(object)),
            _ => unreachable!("invalid object type {object_type} for struct {}", object.get("name").unwrap())
        }
    }
}

impl LoadFromSerdeJSON for Version {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let version_introduced = object.get("version_introduced").map(|v| v.as_number().unwrap_or_else(|| panic!("expected version_introduced to be a number")).as_u64().unwrap() as usize).unwrap_or(1);
        let version_supported = object.get("version_supported").map(|v| v.as_number().unwrap_or_else(|| panic!("expected version_supported to be a number")).as_u64().unwrap() as usize).unwrap_or(version_introduced);
        Self {
            version_introduced,
            version_supported
        }
    }
}

impl LoadFromSerdeJSON for Flags {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let get_flag = |flag: &str| {
            object.get(flag).map(|f| f.as_bool().unwrap_or_else(|| panic!("expected {flag} to be a boolean"))).unwrap_or_default()
        };

        Flags {
            non_cached: get_flag("non_cached"),
            cache_only: get_flag("cache_only"),
            uneditable_in_editor: get_flag("read_only"),
            hidden_in_editor: get_flag("hidden"),
            unusable: get_flag("disabled"),
            version: Version::load_from_json(object)
        }
    }
}

impl LoadFromSerdeJSON for StructField {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let field_type = StructFieldType::load_from_json(object);
        let object_type = match &field_type {
            StructFieldType::Object(o) => o,
            StructFieldType::Padding(_) => return Self {
                name: String::new(),
                count: FieldCount::One,
                default_value: None,
                field_type,
                flags: Flags::default(),
                little_endian_in_tags: false,
                maximum: None,
                minimum: None,
                limit: None
            }
        };

        let name = oget_str!(object, "name").to_owned();
        let count = FieldCount::load_from_json(object);

        let parse_static_value = |v: &Value| -> StaticValue {
            let primitive_value_type = object_type.primitive_value_type().unwrap_or_else(|| panic!("{} does not have a primitive value type", oget_str!(object, "type")));
            match primitive_value_type {
                StaticValue::F32(_) => StaticValue::F32(v.as_f64().map(|f| f as f32).unwrap_or_else(|| panic!("expected float for {name}, got {v:?}"))),
                StaticValue::String(_) => StaticValue::String(v.as_str().unwrap_or_else(|| panic!("expected string for {name}, got {v:?}")).to_owned()),
                StaticValue::Int(_) => StaticValue::Int(v.as_i64().unwrap_or_else(|| panic!("expected i64 for {name}, got {v:?}"))),
                StaticValue::Uint(_) => StaticValue::Uint(v.as_u64().unwrap_or_else(|| panic!("expected u64 for {name}, got {v:?}"))),
            }
        };

        let get_static_value = |field_name: &str| -> Option<StaticValue> {
            let o = object.get(field_name)?;
            Some(parse_static_value(o))
        };

        let get_static_values = |field_name: &str| -> Option<Vec<StaticValue>> {
            let o = object.get(field_name)?;

            let parse_static_values = |v: &[Value]| -> Vec<StaticValue> {
                v.iter().map(|v| parse_static_value(v)).collect()
            };

            let result = match o {
                Value::Array(a) => parse_static_values(a.as_slice()),
                _ => parse_static_values(&[o.to_owned()])
            };

            let field_count = count.field_count();
            let expected_default_count = field_count * object_type.composite_count();
            assert_eq!(expected_default_count, result.len(), "count for {name}::{field_name} was {} when it should be {}", result.len(), expected_default_count);
            Some(result)
        };

        StructField {
            minimum: get_static_value("minimum"),
            maximum: get_static_value("maximum"),
            limit: None, // TODO
            flags: Flags::load_from_json(object),
            little_endian_in_tags: object.get("little_endian").map(|f| f.as_bool().unwrap()).unwrap_or(false),
            default_value: get_static_values("default"),
            field_type,
            count,
            name,
        }
    }
}

impl LoadFromSerdeJSON for TagReference {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        Self {
            allowed_groups: oget!(object, "groups")
                .as_array()
                .unwrap_or_else(|| panic!("expected {name}::groups to be an array", name=oget_name!(object)))
                .iter()
                .map(|f| f.as_str().unwrap_or_else(|| panic!("expected {name}::groups to only have strings", name=oget_name!(object))))
                .map(|s| s.to_owned())
                .collect()
        }
    }
}

impl LoadFromSerdeJSON for ObjectType {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let field_type = oget_str!(object, "type");

        match field_type {
            "Reflexive" => Self::Reflexive(oget_str!(object, "struct").to_owned()),
            "TagReference" => Self::TagReference(TagReference::load_from_json(object)),
            "Data" => Self::Data,
            "float" => Self::F32,
            "uint8" => Self::U8,
            "uint16" => Self::U16,
            "uint32" => Self::U32,
            "int8" => Self::I8,
            "int16" => Self::I16,
            "int32" => Self::I32,
            "Angle" => Self::Angle,
            "Rectangle" => Self::Rectangle,
            "Vector2D" => Self::Vector2D,
            "Vector3D" => Self::Vector3D,
            "Euler2D" => Self::Euler2D,
            "Euler3D" => Self::Euler3D,
            "Plane2D" => Self::Plane2D,
            "Plane3D" => Self::Plane3D,
            "Quaternion" => Self::Quaternion,
            "Matrix3x3" => Self::Matrix3x3,
            "ColorRGB" => Self::ColorRGB,
            "ColorARGB" => Self::ColorARGB,
            "ColorARGBInt" => Self::ColorARGBInt,
            "String32" => Self::String32,
            "Pointer" => Self::Pointer,
            "Index" => Self::Index,
            "Point2DInt" => Self::Point2DInt,
            "TagID" => Self::TagID,
            "ScenarioScriptNodeValue" => Self::ScenarioScriptNodeValue,
            n => Self::NamedObject(n.to_owned()),
        }
    }
}

impl LoadFromSerdeJSON for StructFieldType {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        if oget_str!(object, "type") == "pad" {
            Self::Padding(oget_size!(object))
        }
        else {
            Self::Object(ObjectType::load_from_json(object))
        }
    }
}

impl LoadFromSerdeJSON for FieldCount {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let is_bounds = object.get("bounds").is_some_and(|f| f.as_bool().unwrap());
        let count = object.get("count").map(|c| c.as_u64().unwrap() as usize);

        if is_bounds && count.is_some() {
            panic!("{}'s field count is ambiguous (both bounds and count set)", oget_name!(object));
        }

        if is_bounds {
            Self::Bounds
        }
        else if let Some(c) = count {
            Self::Array(c)
        }
        else {
            Self::One
        }
    }
}

impl LoadFromSerdeJSON for Struct {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let name = oget_str!(object, "name").to_owned();

        let mut fields = object.get("fields")
                                                    .unwrap_or_else(|| panic!("object {name} is missing fields"))
                                                    .as_array()
                                                    .unwrap_or_else(|| panic!("object {name}'s fields is not an array"))
                                                    .iter()
                                                    .map(|f| f.as_object().unwrap_or_else(|| panic!("object {name}'s fields contains non-objects")))
                                                    .map(|f| StructField::load_from_json(f))
                                                    .collect::<VecDeque<StructField>>();

        if let Some(parent) = object.get("inherits").map(|p| p.as_str().unwrap().to_owned()) {
            fields.push_front(StructField {
                name: "parent".to_owned(),
                count: FieldCount::One,
                field_type: StructFieldType::Object(ObjectType::NamedObject(parent)),
                default_value: None,
                minimum: None,
                maximum: None,
                limit: None,
                flags: Flags::default(),
                little_endian_in_tags: false
            })
        }

        Self {
            flags: Flags::load_from_json(object),
            fields: Vec::from(fields),
            name,
            size: oget_number!(object, "size", as_u64) as usize
        }
    }
}

impl LoadFromSerdeJSON for Field {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        Self {
            name: oget_str!(object, "name").to_owned(),
            flags: Flags::load_from_json(object)
        }
    }
}

fn process_field_array(fields: &Vec<Value>) -> Vec<Field> {
    fields.iter()
        .map(|f| match f {
            Value::String(name) => Field {
                name: name.to_owned(),
                flags: Flags::default()
            },
            Value::Object(o) => Field::load_from_json(o),
            _ => panic!("bitfield/enum entries must be a string or object")
        })
        .collect()
}

impl LoadFromSerdeJSON for Bitfield {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let name = oget_str!(object, "name").to_owned();

        Self {
            width: oget_number!(object, "width", as_u64) as u8,
            flags: Flags::load_from_json(object),
            fields: process_field_array(oget!(object, "fields").as_array().unwrap_or_else(|| panic!("{name}::fields must be an array"))),
            name
        }
    }
}

impl LoadFromSerdeJSON for Enum {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let name = oget_str!(object, "name").to_owned();

        Self {
            flags: Flags::load_from_json(object),
            options: process_field_array(oget!(object, "options").as_array().unwrap_or_else(|| panic!("{name}::options must be an array"))),
            name
        }
    }
}
