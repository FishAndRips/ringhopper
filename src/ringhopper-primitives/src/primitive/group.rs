use std::cmp::Ordering;
use crate::error::*;
use std::fmt::Display;
use std::convert::{TryFrom, TryInto};
use crate::parse::{SimplePrimitive, SimpleTagData};
use byteorder::ByteOrder;
use crate::dynamic::SimplePrimitiveType;

/// Tag groups are serialized as a 32-bit FourCC integer.
pub type FourCC = u32;

/// Refers to a tag group, or a type of tag.
#[derive(Copy, Clone, PartialEq, Debug, Eq, Hash, Default)]
pub enum TagGroup {
    Actor,
    ActorVariant,
    Antenna,
    Biped,
    Bitmap,
    CameraTrack,
    ColorTable,
    ContinuousDamageEffect,
    Contrail,
    DamageEffect,
    Decal,
    DetailObjectCollection,
    Device,
    DeviceControl,
    DeviceLightFixture,
    DeviceMachine,
    Dialogue,
    Effect,
    Equipment,
    Flag,
    Fog,
    Font,
    Garbage,
    GBXModel,
    Globals,
    Glow,
    GrenadeHUDInterface,
    HUDGlobals,
    HUDMessageText,
    HUDNumber,
    InputDeviceDefaults,
    Item,
    ItemCollection,
    LensFlare,
    Light,
    LightVolume,
    Lightning,
    MaterialEffects,
    Meter,
    Model,
    ModelAnimations,
    ModelCollisionGeometry,
    MultiplayerScenarioDescription,
    Object,
    Particle,
    ParticleSystem,
    Physics,
    Placeholder,
    PointPhysics,
    PreferencesNetworkGame,
    Projectile,
    Scenario,
    ScenarioStructureBSP,
    Scenery,
    Shader,
    ShaderEnvironment,
    ShaderModel,
    ShaderTransparentChicago,
    ShaderTransparentChicagoExtended,
    ShaderTransparentGeneric,
    ShaderTransparentGlass,
    ShaderTransparentMeter,
    ShaderTransparentPlasma,
    ShaderTransparentWater,
    Sky,
    Sound,
    SoundEnvironment,
    SoundLooping,
    SoundScenery,
    Spheroid,
    StringList,
    TagCollection,
    UIWidgetCollection,
    UIWidgetDefinition,
    UnicodeStringList,
    Unit,
    UnitHUDInterface,
    VectorFont,
    VectorFontData,
    Vehicle,
    VirtualKeyboard,
    Weapon,
    WeaponHUDInterface,
    WeatherParticleSystem,
    Wind,

    /// Denotes the state of the tag group not being set.
    ///
    /// This is considered invalid at runtime, but some fields that use tag groups may not always have a tag group set.
    #[default]
    _Unset
}

impl Ord for TagGroup {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl PartialOrd for TagGroup {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// All tag groups for CE sorted alphabetically to allow for efficient binary searching.
const ALL_GROUPS: &'static [(&'static str, TagGroup, FourCC)] = &[
    ("actor", TagGroup::Actor, 0x61637472),
    ("actor_variant", TagGroup::ActorVariant, 0x61637476),
    ("antenna", TagGroup::Antenna, 0x616E7421),
    ("biped", TagGroup::Biped, 0x62697064),
    ("bitmap", TagGroup::Bitmap, 0x6269746D),
    ("camera_track", TagGroup::CameraTrack, 0x7472616B),
    ("color_table", TagGroup::ColorTable, 0x636F6C6F),
    ("continuous_damage_effect", TagGroup::ContinuousDamageEffect, 0x63646D67),
    ("contrail", TagGroup::Contrail, 0x636F6E74),
    ("damage_effect", TagGroup::DamageEffect, 0x6A707421),
    ("decal", TagGroup::Decal, 0x64656361),
    ("detail_object_collection", TagGroup::DetailObjectCollection, 0x646F6263),
    ("device", TagGroup::Device, 0x64657669),
    ("device_control", TagGroup::DeviceControl, 0x6374726C),
    ("device_light_fixture", TagGroup::DeviceLightFixture, 0x6C696669),
    ("device_machine", TagGroup::DeviceMachine, 0x6D616368),
    ("dialogue", TagGroup::Dialogue, 0x75646C67),
    ("effect", TagGroup::Effect, 0x65666665),
    ("equipment", TagGroup::Equipment, 0x65716970),
    ("flag", TagGroup::Flag, 0x666C6167),
    ("fog", TagGroup::Fog, 0x666F6720),
    ("font", TagGroup::Font, 0x666F6E74),
    ("garbage", TagGroup::Garbage, 0x67617262),
    ("gbxmodel", TagGroup::GBXModel, 0x6D6F6432),
    ("globals", TagGroup::Globals, 0x6D617467),
    ("glow", TagGroup::Glow, 0x676C7721),
    ("grenade_hud_interface", TagGroup::GrenadeHUDInterface, 0x67726869),
    ("hud_globals", TagGroup::HUDGlobals, 0x68756467),
    ("hud_message_text", TagGroup::HUDMessageText, 0x686D7420),
    ("hud_number", TagGroup::HUDNumber, 0x68756423),
    ("input_device_defaults", TagGroup::InputDeviceDefaults, 0x64657663),
    ("item", TagGroup::Item, 0x6974656D),
    ("item_collection", TagGroup::ItemCollection, 0x69746D63),
    ("lens_flare", TagGroup::LensFlare, 0x6C656E73),
    ("light", TagGroup::Light, 0x6C696768),
    ("light_volume", TagGroup::LightVolume, 0x6D677332),
    ("lightning", TagGroup::Lightning, 0x656C6563),
    ("material_effects", TagGroup::MaterialEffects, 0x666F6F74),
    ("meter", TagGroup::Meter, 0x6D657472),
    ("model", TagGroup::Model, 0x6D6F6465),
    ("model_animations", TagGroup::ModelAnimations, 0x616E7472),
    ("model_collision_geometry", TagGroup::ModelCollisionGeometry, 0x636F6C6C),
    ("multiplayer_scenario_description", TagGroup::MultiplayerScenarioDescription, 0x6D706C79),
    ("object", TagGroup::Object, 0x6F626A65),
    ("particle", TagGroup::Particle, 0x70617274),
    ("particle_system", TagGroup::ParticleSystem, 0x7063746C),
    ("physics", TagGroup::Physics, 0x70687973),
    ("placeholder", TagGroup::Placeholder, 0x706C6163),
    ("point_physics", TagGroup::PointPhysics, 0x70706879),
    ("preferences_network_game", TagGroup::PreferencesNetworkGame, 0x6E677072),
    ("projectile", TagGroup::Projectile, 0x70726F6A),
    ("scenario", TagGroup::Scenario, 0x73636E72),
    ("scenario_structure_bsp", TagGroup::ScenarioStructureBSP, 0x73627370),
    ("scenery", TagGroup::Scenery, 0x7363656E),
    ("shader", TagGroup::Shader, 0x73686472),
    ("shader_environment", TagGroup::ShaderEnvironment, 0x73656E76),
    ("shader_model", TagGroup::ShaderModel, 0x736F736F),
    ("shader_transparent_chicago", TagGroup::ShaderTransparentChicago, 0x73636869),
    ("shader_transparent_chicago_extended", TagGroup::ShaderTransparentChicagoExtended, 0x73636578),
    ("shader_transparent_generic", TagGroup::ShaderTransparentGeneric, 0x736F7472),
    ("shader_transparent_glass", TagGroup::ShaderTransparentGlass, 0x73676C61),
    ("shader_transparent_meter", TagGroup::ShaderTransparentMeter, 0x736D6574),
    ("shader_transparent_plasma", TagGroup::ShaderTransparentPlasma, 0x73706C61),
    ("shader_transparent_water", TagGroup::ShaderTransparentWater, 0x73776174),
    ("sky", TagGroup::Sky, 0x736B7920),
    ("sound", TagGroup::Sound, 0x736E6421),
    ("sound_environment", TagGroup::SoundEnvironment, 0x736E6465),
    ("sound_looping", TagGroup::SoundLooping, 0x6C736E64),
    ("sound_scenery", TagGroup::SoundScenery, 0x73736365),
    ("spheroid", TagGroup::Spheroid, 0x626F6F6D),
    ("string_list", TagGroup::StringList, 0x73747223),
    ("tag_collection", TagGroup::TagCollection, 0x74616763),
    ("ui_widget_collection", TagGroup::UIWidgetCollection, 0x536F756C),
    ("ui_widget_definition", TagGroup::UIWidgetDefinition, 0x44654C61),
    ("unicode_string_list", TagGroup::UnicodeStringList, 0x75737472),
    ("unit", TagGroup::Unit, 0x756E6974),
    ("unit_hud_interface", TagGroup::UnitHUDInterface, 0x756E6869),
    ("vector_font", TagGroup::VectorFont, 0x76666E74),
    ("vector_font_data", TagGroup::VectorFontData, 0x76666E64),
    ("vehicle", TagGroup::Vehicle, 0x76656869),
    ("virtual_keyboard", TagGroup::VirtualKeyboard, 0x76636B79),
    ("weapon", TagGroup::Weapon, 0x77656170),
    ("weapon_hud_interface", TagGroup::WeaponHUDInterface, 0x77706869),
    ("weather_particle_system", TagGroup::WeatherParticleSystem, 0x7261696E),
    ("wind", TagGroup::Wind, 0x77696E64),
    ("<unset>", TagGroup::_Unset, 0x00000000),
];

impl TagGroup {
    /// Get the string value of the tag group.
    ///
    /// This is, for example, used for file extensions.
    pub const fn as_str(&self) -> &'static str {
        ALL_GROUPS[*self as usize].0
    }

    /// Convert the string value to a tag group if it exists.
    ///
    /// Returns an error `Error::NoSuchTagGroup` if `str` doesn't correspond to a group.
    pub fn from_str(str: &str) -> RinghopperResult<TagGroup> {
        ALL_GROUPS.binary_search_by(|probe| probe.0.cmp(str))
            .map(|n| ALL_GROUPS[n].1)
            .map_err(|_| Error::InvalidFourCC)
    }

    /// Get the FourCC value of the tag group.
    pub const fn as_fourcc(&self) -> FourCC {
        ALL_GROUPS[*self as usize].2
    }

    /// Convert the `FourCC` value to a tag group if it exists.
    ///
    /// Returns `None` if no such tag group exists.
    pub const fn from_fourcc(fourcc: FourCC) -> Option<TagGroup> {
        let mut i = 0;
        while i < ALL_GROUPS.len() {
            let group = &ALL_GROUPS[i];
            if group.2 == fourcc {
                return Some(group.1)
            }
            i += 1;
        }
        if fourcc == u32::MAX {
            return Some(TagGroup::_Unset);
        }
        None
    }

    /// Get the group this tag group supergroups, if there is one.
    ///
    /// Returns `None` if no such tag group exists.
    pub const fn subgroup(self) -> Option<TagGroup> {
        match self {
            TagGroup::Unit
            | TagGroup::Item
            | TagGroup::Device => Some(TagGroup::Object),

            TagGroup::Biped
            | TagGroup::Vehicle => Some(TagGroup::Unit),

            TagGroup::Weapon
            | TagGroup::Garbage
            | TagGroup::Equipment => Some(TagGroup::Item),

            TagGroup::DeviceMachine
            | TagGroup::DeviceControl
            | TagGroup::DeviceLightFixture => Some(TagGroup::Device),

            TagGroup::Projectile
            | TagGroup::Scenery
            | TagGroup::Placeholder
            | TagGroup::SoundScenery => Some(TagGroup::Object),

            TagGroup::ShaderModel
            | TagGroup::ShaderEnvironment
            | TagGroup::ShaderTransparentChicago
            | TagGroup::ShaderTransparentChicagoExtended
            | TagGroup::ShaderTransparentGeneric
            | TagGroup::ShaderTransparentGlass
            | TagGroup::ShaderTransparentMeter
            | TagGroup::ShaderTransparentPlasma
            | TagGroup::ShaderTransparentWater => Some(TagGroup::Shader),

            _ => None
        }
    }

    /// Get all subgroups for this tag.
    ///
    /// Any unset tag group will be [`TagGroup::_Unset`].
    pub const fn full_subgroup_tree(self) -> [TagGroup; 3] {
        let primary = self;
        let (secondary, tertiary) = match primary.subgroup() {
            Some(secondary) => (secondary, match secondary.subgroup() {
                Some(tertiary) => tertiary,
                None => TagGroup::_Unset
            }),
            None => (TagGroup::_Unset, TagGroup::_Unset)
        };
        [primary, secondary, tertiary]
    }
}

impl Display for TagGroup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<FourCC> for TagGroup {
    type Error = Error;
    fn try_from(value: FourCC) -> RinghopperResult<Self> {
        TagGroup::from_fourcc(value).ok_or(Error::InvalidFourCC)
    }
}

impl SimpleTagData for TagGroup {
    fn simple_size() -> usize {
        std::mem::size_of::<FourCC>()
    }
    fn read<B: ByteOrder>(data: &[u8], at: usize, struct_end: usize) -> RinghopperResult<Self> {
        u32::read::<B>(data, at, struct_end)?.try_into()
    }
    fn write<B: ByteOrder>(&self, data: &mut [u8], at: usize, struct_end: usize) -> RinghopperResult<()> {
        let v: u32 = (*self).as_fourcc();
        v.write::<B>(data, at, struct_end)
    }
}

impl SimplePrimitive for TagGroup {
    fn primitive_type() -> SimplePrimitiveType {
        SimplePrimitiveType::TagGroup
    }
}
