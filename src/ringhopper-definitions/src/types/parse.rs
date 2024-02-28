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
        $obj.get($field).unwrap_or_else(|| panic!("no such field `{name}::{field}`", field=$field, name=oget_name!($obj)))
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

impl ParsedDefinitions {
    pub(crate) fn load_from_json(&mut self, objects: &Vec<Map<String, Value>>) {
        let mut all_engines = HashMap::<String, Map<String, Value>>::new();

        for object in objects {
            let object_type = oget_str!(object, "type");
            let object_name = oget_str!(object, "name").to_owned();
            assert!(!object_name.is_empty());

            match object_type {
                "group" => {
                    assert!(!self.groups.contains_key(&object_name), "duplicate group {object_name} detected");
                    let parent_maybe = object.get("supergroup").map(|g| g.as_str().unwrap().to_owned());
                    self.groups.insert(object_name.clone(), TagGroup {
                        struct_name: oget_str!(object, "struct").to_owned(),
                        supergroup: parent_maybe,
                        supported_engines: SupportedEngines::load_from_json(object),
                        version: oget_number!(object, "version", as_u64).try_into().unwrap_or_else(|e| panic!("{object_name}::version can't convert to u16: {e}")),
                        name: object_name,
                    });
                },
                "engine" => {
                    assert!(!all_engines.contains_key(&object_name), "duplicate engine {object_name} detected");
                    all_engines.insert(object_name, object.clone());
                },
                _ => {
                    assert!(!self.objects.contains_key(&object_name), "duplicate object {object_name} detected");
                    self.objects.insert(object_name, NamedObject::load_from_json(object));
                }
            }
        }

        for (engine_name, _) in &all_engines {
            // Values are ("engine::value", value)
            fn get_chain(what: &str, engine_name: &str, all_engines: &HashMap<String, Map<String, Value>>) -> Vec<(String, Value)> {
                let mut v: Vec<(String, Value)> = Vec::new();
                let engine = all_engines.get(engine_name).unwrap_or_else(|| panic!("can't find engine {engine_name}"));
                if let Some(n) = engine.get(what) {
                    v.push((format!("{engine_name}::{what}"), n.to_owned()))
                }
                if let Some(i) = engine.get("inherits") {
                    v.append(
                        &mut get_chain(what, i.as_str().unwrap_or_else(|| panic!("inherits of {engine_name} is non-string")), all_engines)
                    );
                }
                v
            }

            let get_chain = |what: &str, required: bool| -> Vec<(String, Value)> {
                let result = get_chain(what, engine_name, &all_engines);
                if required && result.is_empty() {
                    panic!("{what} is not present in {engine_name} or its ancestors")
                }
                result
            };

            let hex_to_u64 = |hex: &Value| -> Option<u64> {
                let str = hex.as_str()?;
                if !str.starts_with("0x") {
                    return None
                }
                u64::from_str_radix(&str[2..], 16).ok()
            };

            let parse_hex_u64 = |what: Vec<(String, Value)>| -> Vec<(String, u64)> {
                what.iter()
                    .map(|(f, v)| {
                        let val = hex_to_u64(v).unwrap_or_else(|| panic!("{f} could not be parsed as hex"));
                        (f.to_owned(), val)
                    })
                    .collect()
            };

            let first_string = |what: &str, required: bool| get_chain(what, required).first().map(|(f, v)| v.as_str().unwrap_or_else(|| panic!("{f} is nonstring")).to_owned());
            let first_u64 = |what: &str, required: bool| get_chain(what, required).first().map(|(f, v)| v.as_u64().unwrap_or_else(|| panic!("{f} is non-u64")).to_owned());
            let first_bool = |what: &str, required: bool| get_chain(what, required).first().map(|(f, v)| v.as_bool().unwrap_or_else(|| panic!("{f} is non-bool")).to_owned());

            let base_memory_address = {
                let bma_search = get_chain("base_memory_address", true);
                let (bma_path, bma_obj) = bma_search.first().unwrap();

                let bma_address_obj: &Value;
                let bma_inferred_obj: &Value;

                match bma_obj {
                    Value::Object(o) => {
                        bma_address_obj = o.get("value").unwrap_or_else(|| panic!("{bma_path} has no address"));
                        bma_inferred_obj = o.get("inferred").unwrap_or(&Value::Bool(false));
                    },
                    Value::String(_) => {
                        bma_address_obj = bma_obj;
                        bma_inferred_obj = &Value::Bool(false);
                    },
                    _ => panic!("{bma_path} is not object or string")
                }

                BaseMemoryAddress {
                    address: hex_to_u64(bma_address_obj).unwrap_or_else(|| panic!("{bma_path}'s address is nonhex")),
                    inferred: bma_inferred_obj.as_bool().unwrap_or_else(|| panic!("{bma_path} strict is non-bool"))
                }
            };

            let max_cache_file_size = {
                let cfz_search = get_chain("max_cache_file_size", true);
                let (cfz_path, cfz_obj) = cfz_search.first().unwrap();

                let multiplayer: &Value;
                let singleplayer: &Value;
                let user_interface: &Value;

                match cfz_obj {
                    Value::Object(o) => {
                        multiplayer = o.get("multiplayer").unwrap_or_else(|| panic!("{cfz_path} has no multiplayer"));
                        singleplayer = o.get("singleplayer").unwrap_or_else(|| panic!("{cfz_path} has no singleplayer"));
                        user_interface = o.get("user_interface").unwrap_or_else(|| panic!("{cfz_path} has no user_interface"));
                    },
                    Value::String(_) => {
                        multiplayer = cfz_obj;
                        singleplayer = cfz_obj;
                        user_interface = cfz_obj;
                    },
                    _ => panic!("{cfz_path} is not object or string")
                }

                EngineCacheFileSize {
                    multiplayer: hex_to_u64(multiplayer).unwrap_or_else(|| panic!("{cfz_path} multiplayer is not hex")),
                    singleplayer: hex_to_u64(singleplayer).unwrap_or_else(|| panic!("{cfz_path} singleplayer is not hex")),
                    user_interface: hex_to_u64(user_interface).unwrap_or_else(|| panic!("{cfz_path} user_interface is not hex")),
                }
            };

            let required_tags = {
                let ert = get_chain("required_tags", true);
                let ert = ert.iter().map(|(k, v)| (k.to_owned(), v.as_object().unwrap_or_else(|| panic!("{k} is non-object"))));

                let mut required_tags = EngineRequiredTags::default();

                for obj in ert {
                    let handler = |tags: &mut Vec<String>, what: &str| {
                        let list = match obj.1.get(what) { Some(n) => n, None => return };
                        let list = list.as_array().unwrap_or_else(|| panic!("{engine_name}::{what} is not an array"));
                        tags.reserve(list.len());
                        for i in list {
                            tags.push(i.as_str().unwrap_or_else(|| panic!("{engine_name}::{what} contains non-strings")).to_owned());
                        }
                    };

                    handler(&mut required_tags.all, "all");
                    handler(&mut required_tags.user_interface, "user_interface");
                    handler(&mut required_tags.singleplayer, "singleplayer");
                    handler(&mut required_tags.multiplayer, "multiplayer");
                }

                required_tags.all.dedup();
                required_tags.multiplayer.dedup();
                required_tags.singleplayer.dedup();
                required_tags.user_interface.dedup();

                required_tags
            };

            self.engines.insert(engine_name.to_owned(), Engine {
                base_memory_address,
                build: first_string("build", false),
                build_target: first_bool("build_target", true).unwrap(),
                cache_file_version: first_u64("cache_file_version", true).unwrap().try_into().unwrap_or_else(|_| panic!("where's the cache file version???")),
                display_name: first_string("display_name", true).unwrap(),
                inherits: get_chain("inherits", false).first().map(|v| v.1.as_str().unwrap().to_owned()),
                max_cache_file_size,
                max_script_nodes: first_u64("max_script_nodes", true).unwrap() as u64,
                max_tag_space: parse_hex_u64(get_chain("max_tag_space", true)).first().unwrap().1,
                name: engine_name.to_owned(),
                required_tags,
                version: first_string("version", false)
            });
        }


    }

    // Fix all tag references to have child groups
    pub(crate) fn resolve_parent_class_references(&mut self) {
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

    pub(crate) fn assert_valid(&self) {
        let validate_supported_engines = |supported_engines: &SupportedEngines, object_name: &str, field_name: &str| {
            if let Some(v) = &supported_engines.supported_engines {
                for engine in v {
                    if !self.engines.contains_key(engine) {
                        panic!("{object_name}::{field_name}'s limits references an engine {engine} which does not exist");
                    }
                }
            }
        };

        for (group_name, group) in &self.groups {
            let group_name_in_struct = &group.name;
            assert_eq!(group_name_in_struct, group_name, "group name `{group_name_in_struct}` not consistent with name `{group_name}` in map");

            let struct_name = &group.struct_name;
            self.objects.get(struct_name).unwrap_or_else(|| panic!("group {group_name} refers to struct {struct_name} which does not exist"));

            if let Some(s) = &group.supergroup {
                self.groups.get(s).unwrap_or_else(|| panic!("group {group_name}'s supergroup refers to group {s} which does not exist"));
            }

            validate_supported_engines(&group.supported_engines, &group_name, "(self)");
        }

        for (object_name, object) in &self.objects {
            let name_in_object = object.name();
            assert_eq!(name_in_object, object_name, "object name `{name_in_object}` not consistent with name `{object_name}` in map");

            let validate_flags = |flags: &Flags, field_name: &str| validate_supported_engines(&flags.supported_engines, &object_name, field_name);

            match object {
                NamedObject::Bitfield(b) => {
                    validate_flags(&b.flags, "(self)");

                    for f in &b.fields {
                        validate_flags(&f.flags, &f.name);
                    }

                    for i in 0..b.fields.len() {
                        for j in i+1..b.fields.len() {
                            let field_name = &b.fields[i].name;
                            assert_ne!(field_name, &b.fields[j].name, "bitfield {object_name} has duplicate fields {field_name}");
                        }
                    }

                    assert!(b.fields.len() <= b.width as usize, "bitfield {object_name} has too many fields; {} / {}", b.fields.len(), b.width);
                },
                NamedObject::Enum(e) => {
                    validate_flags(&e.flags, "(self)");

                    for f in &e.options {
                        validate_flags(&f.flags, &f.name);
                    }

                    for i in 0..e.options.len() {
                        for j in i+1..e.options.len() {
                            let option_name = &e.options[i].name;
                            assert_ne!(option_name, &e.options[j].name, "enum {object_name} has duplicate options {option_name}");
                        }
                    }

                    assert!(e.options.len() <= u16::MAX as usize, "enum {object_name} has too many options, {} / {}", e.options.len(), u16::MAX);
                },
                NamedObject::Struct(s) => {
                    validate_flags(&s.flags, "(self)");

                    for i in 0..s.fields.len() {
                        if matches!(s.fields[i].field_type, StructFieldType::Padding(_) | StructFieldType::EditorSection(_)) {
                            continue
                        }
                        for j in i+1..s.fields.len() {
                            if matches!(s.fields[j].field_type, StructFieldType::Padding(_) | StructFieldType::EditorSection(_)) {
                                continue
                            }
                            let field_name = &s.fields[i].name;
                            assert_ne!(field_name, &s.fields[j].name, "struct {object_name} has duplicate fields {field_name}");
                        }
                    }

                    for f in &s.fields {
                        // Consistency with named objects and groups
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

                        // Limits point to engines
                        if let Some(n) = &f.limit {
                            for (k, _) in n {
                                if let LimitType::Engine(e) = k {
                                    if !self.engines.contains_key(e) {
                                        panic!("{object_name}::{field_name}'s limits contains an engine {e} which does not exist");
                                    }
                                }
                            }
                        }

                        validate_flags(&f.flags, &field_name);
                    }

                    s.assert_size_is_correct(self);
                }
            }
        }
    }
}

pub(crate) fn get_all_definitions() -> Vec<Map<String, Value>> {
    let mut jsons: HashMap<&'static str, &'static [u8]> = HashMap::new();

    jsons.insert("tag/actor_variant.json", include_bytes!("../../json/tag/actor_variant.json"));
    jsons.insert("tag/actor.json", include_bytes!("../../json/tag/actor.json"));
    jsons.insert("tag/antenna.json", include_bytes!("../../json/tag/antenna.json"));
    jsons.insert("tag/biped.json", include_bytes!("../../json/tag/biped.json"));
    jsons.insert("tag/bitfield.json", include_bytes!("../../json/tag/bitfield.json"));
    jsons.insert("tag/bitmap.json", include_bytes!("../../json/tag/bitmap.json"));
    jsons.insert("tag/camera_track.json", include_bytes!("../../json/tag/camera_track.json"));
    jsons.insert("tag/color_table.json", include_bytes!("../../json/tag/color_table.json"));
    jsons.insert("tag/continuous_damage_effect.json", include_bytes!("../../json/tag/continuous_damage_effect.json"));
    jsons.insert("tag/contrail.json", include_bytes!("../../json/tag/contrail.json"));
    jsons.insert("tag/damage_effect.json", include_bytes!("../../json/tag/damage_effect.json"));
    jsons.insert("tag/decal.json", include_bytes!("../../json/tag/decal.json"));
    jsons.insert("tag/detail_object_collection.json", include_bytes!("../../json/tag/detail_object_collection.json"));
    jsons.insert("tag/device_control.json", include_bytes!("../../json/tag/device_control.json"));
    jsons.insert("tag/device_light_fixture.json", include_bytes!("../../json/tag/device_light_fixture.json"));
    jsons.insert("tag/device_machine.json", include_bytes!("../../json/tag/device_machine.json"));
    jsons.insert("tag/device.json", include_bytes!("../../json/tag/device.json"));
    jsons.insert("tag/dialogue.json", include_bytes!("../../json/tag/dialogue.json"));
    jsons.insert("tag/effect.json", include_bytes!("../../json/tag/effect.json"));
    jsons.insert("tag/enum.json", include_bytes!("../../json/tag/enum.json"));
    jsons.insert("tag/equipment.json", include_bytes!("../../json/tag/equipment.json"));
    jsons.insert("tag/flag.json", include_bytes!("../../json/tag/flag.json"));
    jsons.insert("tag/fog.json", include_bytes!("../../json/tag/fog.json"));
    jsons.insert("tag/font.json", include_bytes!("../../json/tag/font.json"));
    jsons.insert("tag/garbage.json", include_bytes!("../../json/tag/garbage.json"));
    jsons.insert("tag/gbxmodel.json", include_bytes!("../../json/tag/gbxmodel.json"));
    jsons.insert("tag/globals.json", include_bytes!("../../json/tag/globals.json"));
    jsons.insert("tag/glow.json", include_bytes!("../../json/tag/glow.json"));
    jsons.insert("tag/grenade_hud_interface.json", include_bytes!("../../json/tag/grenade_hud_interface.json"));
    jsons.insert("tag/hud_globals.json", include_bytes!("../../json/tag/hud_globals.json"));
    jsons.insert("tag/hud_interface_types.json", include_bytes!("../../json/tag/hud_interface_types.json"));
    jsons.insert("tag/hud_message_text.json", include_bytes!("../../json/tag/hud_message_text.json"));
    jsons.insert("tag/hud_number.json", include_bytes!("../../json/tag/hud_number.json"));
    jsons.insert("tag/input_device_defaults.json", include_bytes!("../../json/tag/input_device_defaults.json"));
    jsons.insert("tag/item_collection.json", include_bytes!("../../json/tag/item_collection.json"));
    jsons.insert("tag/item.json", include_bytes!("../../json/tag/item.json"));
    jsons.insert("tag/lens_flare.json", include_bytes!("../../json/tag/lens_flare.json"));
    jsons.insert("tag/light_volume.json", include_bytes!("../../json/tag/light_volume.json"));
    jsons.insert("tag/light.json", include_bytes!("../../json/tag/light.json"));
    jsons.insert("tag/lightning.json", include_bytes!("../../json/tag/lightning.json"));
    jsons.insert("tag/material_effects.json", include_bytes!("../../json/tag/material_effects.json"));
    jsons.insert("tag/meter.json", include_bytes!("../../json/tag/meter.json"));
    jsons.insert("tag/model_animations.json", include_bytes!("../../json/tag/model_animations.json"));
    jsons.insert("tag/model_collision_geometry.json", include_bytes!("../../json/tag/model_collision_geometry.json"));
    jsons.insert("tag/model.json", include_bytes!("../../json/tag/model.json"));
    jsons.insert("tag/multiplayer_scenario_description.json", include_bytes!("../../json/tag/multiplayer_scenario_description.json"));
    jsons.insert("tag/object.json", include_bytes!("../../json/tag/object.json"));
    jsons.insert("tag/particle_system.json", include_bytes!("../../json/tag/particle_system.json"));
    jsons.insert("tag/particle.json", include_bytes!("../../json/tag/particle.json"));
    jsons.insert("tag/physics.json", include_bytes!("../../json/tag/physics.json"));
    jsons.insert("tag/placeholder.json", include_bytes!("../../json/tag/placeholder.json"));
    jsons.insert("tag/point_physics.json", include_bytes!("../../json/tag/point_physics.json"));
    jsons.insert("tag/preferences_network_game.json", include_bytes!("../../json/tag/preferences_network_game.json"));
    jsons.insert("tag/projectile.json", include_bytes!("../../json/tag/projectile.json"));
    jsons.insert("tag/scenario_structure_bsp.json", include_bytes!("../../json/tag/scenario_structure_bsp.json"));
    jsons.insert("tag/scenario.json", include_bytes!("../../json/tag/scenario.json"));
    jsons.insert("tag/scenery.json", include_bytes!("../../json/tag/scenery.json"));
    jsons.insert("tag/shader_environment.json", include_bytes!("../../json/tag/shader_environment.json"));
    jsons.insert("tag/shader_model.json", include_bytes!("../../json/tag/shader_model.json"));
    jsons.insert("tag/shader_transparent_chicago_extended.json", include_bytes!("../../json/tag/shader_transparent_chicago_extended.json"));
    jsons.insert("tag/shader_transparent_chicago.json", include_bytes!("../../json/tag/shader_transparent_chicago.json"));
    jsons.insert("tag/shader_transparent_generic.json", include_bytes!("../../json/tag/shader_transparent_generic.json"));
    jsons.insert("tag/shader_transparent_glass.json", include_bytes!("../../json/tag/shader_transparent_glass.json"));
    jsons.insert("tag/shader_transparent_meter.json", include_bytes!("../../json/tag/shader_transparent_meter.json"));
    jsons.insert("tag/shader_transparent_plasma.json", include_bytes!("../../json/tag/shader_transparent_plasma.json"));
    jsons.insert("tag/shader_transparent_water.json", include_bytes!("../../json/tag/shader_transparent_water.json"));
    jsons.insert("tag/shader.json", include_bytes!("../../json/tag/shader.json"));
    jsons.insert("tag/sky.json", include_bytes!("../../json/tag/sky.json"));
    jsons.insert("tag/sound_environment.json", include_bytes!("../../json/tag/sound_environment.json"));
    jsons.insert("tag/sound_looping.json", include_bytes!("../../json/tag/sound_looping.json"));
    jsons.insert("tag/sound_scenery.json", include_bytes!("../../json/tag/sound_scenery.json"));
    jsons.insert("tag/sound.json", include_bytes!("../../json/tag/sound.json"));
    jsons.insert("tag/string_list.json", include_bytes!("../../json/tag/string_list.json"));
    jsons.insert("tag/tag_collection.json", include_bytes!("../../json/tag/tag_collection.json"));
    jsons.insert("tag/ui_widget_collection.json", include_bytes!("../../json/tag/ui_widget_collection.json"));
    jsons.insert("tag/ui_widget_definition.json", include_bytes!("../../json/tag/ui_widget_definition.json"));
    jsons.insert("tag/unicode_string_list.json", include_bytes!("../../json/tag/unicode_string_list.json"));
    jsons.insert("tag/unit_hud_interface.json", include_bytes!("../../json/tag/unit_hud_interface.json"));
    jsons.insert("tag/unit.json", include_bytes!("../../json/tag/unit.json"));
    jsons.insert("tag/vehicle.json", include_bytes!("../../json/tag/vehicle.json"));
    jsons.insert("tag/virtual_keyboard.json", include_bytes!("../../json/tag/virtual_keyboard.json"));
    jsons.insert("tag/weapon_hud_interface.json", include_bytes!("../../json/tag/weapon_hud_interface.json"));
    jsons.insert("tag/weapon.json", include_bytes!("../../json/tag/weapon.json"));
    jsons.insert("tag/weather_particle_system.json", include_bytes!("../../json/tag/weather_particle_system.json"));
    jsons.insert("tag/wind.json", include_bytes!("../../json/tag/wind.json"));

    jsons.insert("map/cache.json", include_bytes!("../../json/map/cache.json"));

    jsons.insert("engine/halo macintosh demo.json", include_bytes!("../../json/engine/halo macintosh demo.json"));
    jsons.insert("engine/halo macintosh retail.json", include_bytes!("../../json/engine/halo macintosh retail.json"));
    jsons.insert("engine/halo mcc cea.json", include_bytes!("../../json/engine/halo mcc cea.json"));
    jsons.insert("engine/halo pc custom edition.json", include_bytes!("../../json/engine/halo pc custom edition.json"));
    jsons.insert("engine/halo pc demo.json", include_bytes!("../../json/engine/halo pc demo.json"));
    jsons.insert("engine/halo pc retail.json", include_bytes!("../../json/engine/halo pc retail.json"));
    jsons.insert("engine/halo pc.json", include_bytes!("../../json/engine/halo pc.json"));
    jsons.insert("engine/halo xbox ntsc demo.json", include_bytes!("../../json/engine/halo xbox ntsc demo.json"));
    jsons.insert("engine/halo xbox ntsc jp.json", include_bytes!("../../json/engine/halo xbox ntsc jp.json"));
    jsons.insert("engine/halo xbox ntsc tw.json", include_bytes!("../../json/engine/halo xbox ntsc tw.json"));
    jsons.insert("engine/halo xbox ntsc us.json", include_bytes!("../../json/engine/halo xbox ntsc us.json"));
    jsons.insert("engine/halo xbox pal.json", include_bytes!("../../json/engine/halo xbox pal.json"));
    jsons.insert("engine/halo xbox.json", include_bytes!("../../json/engine/halo xbox.json"));

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
fn test_load_all_definitions() {
    crate::load_all_definitions();
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

impl LoadFromSerdeJSON for SupportedEngines {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let supported = match object.get("supported_engines") {
            Some(n) => n,
            None => return Self::default()
        };

        let supported_engines = supported.as_array()
            .unwrap_or_else(|| panic!("{}::version is not an array", oget_name!(object)))
            .iter()
            .map(|f| f.as_str().unwrap_or_else(|| panic!("{}::version contains non-strings", oget_name!(object))).to_owned())
            .collect::<Vec<String>>();

        Self { supported_engines: Some(supported_engines) }
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
            exclude: get_flag("exclude"),
            little_endian_in_tags: get_flag("little_endian"),
            supported_engines: SupportedEngines::load_from_json(object),
            shifted_by_one: get_flag("shifted_by_one")
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
                maximum: None,
                minimum: None,
                limit: None
            },
            StructFieldType::EditorSection(e) => return Self {
                name: e.name.clone(),
                count: FieldCount::One,
                default_value: None,
                field_type,
                flags: Flags::default(),
                maximum: None,
                minimum: None,
                limit: None
            },
        };

        let name = oget_str!(object, "name").to_owned();
        assert!(!name.is_empty());
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

        let limit = object.get("limit").map(|l| {
            match l {
                Value::Number(n) => {
                    let mut map = HashMap::new();
                    let limit = n.as_u64().unwrap_or_else(|| panic!("{name}::limit is not u64")) as usize;
                    map.insert(LimitType::Editor, limit);
                    map.insert(LimitType::Default, limit);
                    map
                },

                Value::Object(o) => {
                    let mut map = HashMap::new();

                    let mut editor_limit: Option<usize> = None;
                    let mut default_limit: Option<usize> = None;

                    for (k, v) in o {
                        let v = v.as_number().unwrap_or_else(|| panic!("{name}::limit is are not all numbers"))
                            .as_u64().unwrap_or_else(|| panic!("{name}::limit is not all u64's"))
                            as usize;

                        if k == "default" {
                            default_limit = Some(v);
                        }
                        else {
                            map.insert(LimitType::Engine(k.to_owned()), v);
                        }

                        editor_limit = Some(editor_limit.unwrap_or_default().max(v))
                    }

                    default_limit.unwrap_or_else(|| panic!("No default limit set for {name}"));

                    let editor_limit = editor_limit.unwrap_or_else(|| panic!("Unable to establish an editor limit for {name} (no limits maybe?)"));
                    map.insert(LimitType::Editor, editor_limit);
                    map
                }

                _ => panic!("{name} is not a number or an object")
            }
        });

        StructField {
            minimum: get_static_value("minimum"),
            maximum: get_static_value("maximum"),
            limit,
            flags: Flags::load_from_json(object),
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
            "TagGroup" => Self::TagGroup,
            "Data" => Self::Data,
            "FileData" => Self::FileData,
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
            "CompressedVector3D" => Self::CompressedVector3D,
            "CompressedFloat" => Self::CompressedFloat,
            "Euler2D" => Self::Euler2D,
            "Euler3D" => Self::Euler3D,
            "Plane2D" => Self::Plane2D,
            "Plane3D" => Self::Plane3D,
            "Quaternion" => Self::Quaternion,
            "Matrix3x3" => Self::Matrix3x3,
            "ColorRGBFloat" => Self::ColorRGBFloat,
            "ColorARGBFloat" => Self::ColorARGBFloat,
            "ColorARGBInt" => Self::ColorARGBInt,
            "String32" => Self::String32,
            "Address" => Self::Address,
            "Index" => Self::Index,
            "Vector2DInt" => Self::Vector2DInt,
            "TagID" => Self::TagID,
            "ID" => Self::ID,
            "ScenarioScriptNodeValue" => Self::ScenarioScriptNodeValue,
            n => Self::NamedObject(n.to_owned()),
        }
    }
}

impl LoadFromSerdeJSON for StructFieldType {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        match oget_str!(object, "type") {
            "pad" => Self::Padding(oget_size!(object)),
            "editor_section" => Self::EditorSection(EditorSectionData::load_from_json(object)),
            _ => Self::Object(ObjectType::load_from_json(object))
        }
    }
}

impl LoadFromSerdeJSON for EditorSectionData {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let name = oget_str!(object, "name").to_owned();
        let description = object
            .get("description")
            .map(|d| d.as_str().expect("description must be a string").to_owned());
        Self {
            name, description
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
        assert!(!name.is_empty());

        let flags = Flags::load_from_json(object);

        let mut fields = object.get("fields")
                                                    .unwrap_or_else(|| panic!("object {name} is missing fields"))
                                                    .as_array()
                                                    .unwrap_or_else(|| panic!("object {name}'s fields is not an array"))
                                                    .iter()
                                                    .map(|f| f.as_object().unwrap_or_else(|| panic!("object {name}'s fields contains non-objects")))
                                                    .map(|f| StructField::load_from_json(f))
                                                    .collect::<VecDeque<StructField>>();

        for i in &mut fields {
            i.flags.combine_with(&flags);
        }

        if let Some(parent) = object.get("inherits").map(|p| p.as_str().unwrap().to_owned()) {
            let mut parent_snake_case = String::with_capacity(parent.len() * 2);
            let mut last_char = 'A';
            for c in parent.chars() {
                if c.is_ascii_uppercase() && !last_char.is_ascii_uppercase() {
                    parent_snake_case.push('_');
                }
                last_char = c;
                parent_snake_case.push(c.to_ascii_lowercase());
            }

            fields.push_front(StructField {
                name: parent_snake_case,
                count: FieldCount::One,
                field_type: StructFieldType::Object(ObjectType::NamedObject(parent)),
                default_value: None,
                minimum: None,
                maximum: None,
                limit: None,
                flags: Flags::default()
            })
        }

        Self {
            flags,
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
            flags: Flags::load_from_json(object),
            value: 0
        }
    }
}

fn process_field_array(fields: &Vec<Value>) -> Vec<Field> {
    let mut current_index = 0;

    fields.iter()
        .map(|f| {
            let mut field = match f {
                Value::String(name) => Field {
                    name: name.to_owned(),
                    flags: Flags::default(),
                    value: 0
                },
                Value::Object(o) => Field::load_from_json(o),
                _ => panic!("bitfield/enum entries must be a string or object")
            };

            field.value = current_index;
            current_index += 1;

            field
        })
        .collect()
}

impl LoadFromSerdeJSON for Bitfield {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let name = oget_str!(object, "name").to_owned();
        assert!(!name.is_empty());

        let mut fields = process_field_array(oget!(object, "fields").as_array().unwrap_or_else(|| panic!("{name}::fields must be an array")));
        for f in &mut fields {
            if f.value >= 32 {
                panic!("field {name}::{} is too high to be represented as a bitfield", f.name);
            }
            f.value = 1 << f.value;
        }

        Self {
            width: oget_number!(object, "width", as_u64) as u8,
            flags: Flags::load_from_json(object),
            fields,
            name
        }
    }
}

impl LoadFromSerdeJSON for Enum {
    fn load_from_json(object: &Map<String, Value>) -> Self {
        let name = oget_str!(object, "name").to_owned();
        assert!(!name.is_empty());

        Self {
            flags: Flags::load_from_json(object),
            options: process_field_array(oget!(object, "options").as_array().unwrap_or_else(|| panic!("{name}::options must be an array"))),
            name
        }
    }
}
