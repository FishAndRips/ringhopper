use std::any::Any;
use crate::error::{Error, RinghopperResult};
use crate::parse::TagData;
use crate::primitive::parse_range;

/// Trait for dynamically accessing tag data.
pub trait DynamicTagData: TagData + 'static {
    /// Get the field `field`.
    ///
    /// Returns `None` if the field does not exist.
    fn get_field(&self, field: &str) -> Option<&dyn DynamicTagData>;

    /// Get the field `field` as a mutable reference.
    ///
    /// Returns `None` if the field does not exist.
    fn get_field_mut(&mut self, field: &str) -> Option<&mut dyn DynamicTagData>;

    /// Get all available fields in the object.
    fn fields(&self) -> &'static [&'static str];

    /// Get the `DynamicTagData` object as an `Any` reference.
    fn as_any(&self) -> &dyn Any;

    /// Get the `DynamicTagData` object as a mutable `Any` reference.
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Get the type of data this object is.
    fn data_type(&self) -> DynamicTagDataType;

    /// Get the `DynamicTagData` object as a mutable `DynamicReflexive` reference.
    ///
    /// Returns `None` if this object is not a reflexive.
    fn as_reflexive_mut(&mut self) -> Option<&mut dyn DynamicReflexive> {
        None
    }

    /// Get the `DynamicTagData` object as a `DynamicTagDataArray` reference.
    ///
    /// Returns `None` if this object is not an array.
    fn as_array(&self) -> Option<&dyn DynamicTagDataArray> {
        None
    }

    /// Get the `DynamicTagData` object as a mutable `DynamicTagDataArray` reference.
    ///
    /// Returns `None` if this object is not an array.
    fn as_array_mut(&mut self) -> Option<&mut dyn DynamicTagDataArray> {
        None
    }

    /// Get the `DynamicTagData` object as a `DynamicEnum` reference.
    ///
    /// Returns `None` if this object is not an enum.
    fn as_enum(&self) -> Option<&dyn DynamicEnum> {
        None
    }

    /// Get the `DynamicTagData` object as a mutable `DynamicEnum` reference.
    ///
    /// Returns `None` if this object is not an enum.
    fn as_enum_mut(&mut self) -> Option<&mut dyn DynamicEnum> {
        None
    }
}

/// Trait for dynamically accessing an array of fields, including reflexives.
pub trait DynamicTagDataArray: DynamicTagData {
    /// Get the item at index `index`.
    ///
    /// Returns `None` if the index does not exist.
    fn get_at_index(&self, index: usize) -> Option<&dyn DynamicTagData>;

    /// Get the item at index `index` as a mutable reference.
    ///
    /// Returns `None` if the index does not exist.
    fn get_at_index_mut(&mut self, index: usize) -> Option<&mut dyn DynamicTagData>;

    /// Get the number of items in the array.
    fn len(&self) -> usize;
}

/// Trait for dynamically accessing a reflexive of any type.
pub trait DynamicReflexive: DynamicTagDataArray {
    /// Add an item at index `index` with default values.
    ///
    /// # Panics
    ///
    /// Panics if `index` > `len()`
    fn insert_default(&mut self, index: usize);

    /// Add an item at index `index`, cloning the item.
    ///
    /// # Panics
    ///
    /// Panics if `index` > `len()`
    fn insert_copy(&mut self, index: usize, item: &dyn DynamicTagData);

    /// Add an item at index `index`, moving the contents of `item`.
    ///
    /// # Panics
    ///
    /// Panics if `index` > `len()`
    fn insert_moved(&mut self, index: usize, item: &mut dyn DynamicTagData);
}

#[derive(PartialEq, Debug)]
pub enum DynamicTagDataType {
    Reflexive,
    Array,
    Block,
    Data,
    TagReference,
    Enum,
    SimplePrimitive(SimplePrimitiveType)
}

#[derive(PartialEq, Debug)]
pub enum SimplePrimitiveType {
    Bool,
    String32,
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    F32,

    Angle,
    Vector2D,
    Vector3D,
    Plane2D,
    Plane3D,
    Euler2D,
    Euler3D,
    Quaternion,
    Matrix3x3,
    Vector2DInt,
    Rectangle,

    ColorRGBFloat,
    ColorARGBInt,
    ColorARGBIntBytes,
    ColorARGBFloat,

    Padding,
    ID,
    TagGroup,
    Address,

    Size,
    ScenarioScriptNodeValue,
    TagFileHeader,
    DataC,
    ReflexiveC,
    TagReferenceC,
}

macro_rules! generate_access_function {
    ($dynamic:expr, $callback:expr, $matcher:expr, $function_name:tt, $get_field_function:tt, $as_array_function:tt, $get_from_array_function:tt) => {{
        if $matcher.is_empty() {
            return $callback(Ok($dynamic))
        }

        // end_of_name, next_matcher
        let find_end_of_name = |what: &str| -> (usize, usize) {
            for (index, character) in what.char_indices() {
                match character {
                    '.' => return (index, index + 1),
                    '[' => return (index, index),
                    _ => continue
                }
            }
            (what.len(), what.len())
        };

        let (end_of_name, next_matcher) = find_end_of_name($matcher);

        if end_of_name == 0 {
            let mut iterator = $matcher.char_indices();
            let (_, first_character) = iterator.next().unwrap();
            if first_character == '[' {
                if let Some(arr) = $dynamic.$as_array_function() {
                    let end_index = match iterator.find(|c| c.1 == ']') {
                        Some(n) => n.0,
                        None => return $callback(Err("bad matcher: expected a ] to close a ["))
                    };

                    let start_of_next = match iterator.next() {
                        Some((_, '.')) => end_index + 2,
                        _ => end_index + 1
                    };

                    let matcher_afterwards = &$matcher[start_of_next..];
                    let range = match parse_range(&$matcher[1..end_index], arr.len()) {
                        Ok(n) => n,
                        Err(e) => return $callback(Err(e))
                    };

                    for r in range {
                        for i in r.0..=r.1 {
                            let dynamic = arr.$get_from_array_function(i).unwrap();

                            if !$function_name(dynamic, matcher_afterwards, $callback) {
                                return false
                            }
                        }
                    }
                    return true;
                }
                else {
                    return $callback(Err("bad matcher: type is not subscriptable"))
                }
            }
            else {
                return $callback(Err("bad matcher: cannot start a field name with '.'"))
            }
        }

        let field = &$matcher[0..end_of_name];
        match $dynamic.$get_field_function(field) {
            Some(n) => $function_name(n, &$matcher[next_matcher..], $callback),
            None => $callback(Err("bad matcher: field not found"))
        }
    }};
}

impl dyn DynamicTagData {
    /// Access the object with the given matcher and callback.
    ///
    /// If the callback returns `false`, it will stop.
    ///
    /// # Notes
    ///
    /// If `Err` is passed into the callback, such as due to an invalid matcher, and `true` is returned, this may still
    /// continue. Additionally, you may not immediately get errors until later (in the case the matcher would match more
    /// than one). You can use [`dyn DynamicTagData::validate_matcher`] ahead of time to ensure the matcher is correct
    /// and will not have errors.
    pub fn foreach<F>(&self, matcher: &str, mut callback: F) where F: FnMut(Result<&dyn DynamicTagData, &'static str>) -> bool {
        fn inner<F>(dynamic: &dyn DynamicTagData, matcher: &str, callback: &mut F) -> bool where F: FnMut(Result<&dyn DynamicTagData, &'static str>) -> bool {
            generate_access_function!(dynamic, callback, matcher, inner, get_field, as_array, get_at_index)
        }
        inner(self, matcher, &mut callback);
    }

    /// Mutably access the object with the given matcher and callback.
    ///
    /// If the callback returns `false`, it will stop.
    ///
    /// # Notes
    ///
    /// If `Err` is passed into the callback, such as due to an invalid matcher, and `true` is returned, this may still
    /// continue. Additionally, you may not immediately get errors until later (in the case the matcher would match more
    /// than one). You can use [`dyn DynamicTagData::validate_matcher`] ahead of time to ensure the matcher is correct
    /// and will not have errors.
    pub fn foreach_mut<F>(&mut self, matcher: &str, mut callback: F) where F: FnMut(Result<&mut dyn DynamicTagData, &'static str>) -> bool {
        fn inner<F>(dynamic: &mut dyn DynamicTagData, matcher: &str, callback: &mut F) -> bool where F: FnMut(Result<&mut dyn DynamicTagData, &'static str>) -> bool {
            generate_access_function!(dynamic, callback, matcher, inner, get_field_mut, as_array_mut, get_at_index_mut)
        }
        inner(self, matcher, &mut callback);
    }

    /// Verify the matcher will not have any errors upon being used.
    pub fn validate_matcher(&self, matcher: &str) -> Result<(), &'static str> {
        let mut first_error = Ok(());
        self.foreach(matcher, |r| {
            match r {
                Ok(_) => true,
                Err(e) => {
                    first_error = Err(e);
                    false
                }
            }
        });
        first_error
    }
}

/// Trait for enum objects.
///
/// Implementing this automatically implements [`DynamicEnum`] for the type.
pub trait DynamicEnumImpl {
    /// Get an enum value from a value.
    ///
    /// Returns `None` if the string does not correspond to a valid option.
    fn from_str(value: &str) -> Option<Self> where Self: Sized;

    /// Convert the value to a string.
    fn to_str(&self) -> &'static str;

    /// Retrieve all values for the enum.
    fn str_vals() -> &'static [&'static str];
}

/// Dynamic trait for enum objects.
///
/// # Note
///
/// Implementing [`DynamicEnumImpl`] will automatically implement this.
pub trait DynamicEnum: DynamicTagData {
    /// Overwrite the enum object with the string value.
    fn set_enum_string_value(&mut self, value: &str) -> RinghopperResult<()>;

    /// Convert the enum into its equivalent string value.
    fn get_enum_string_value(&self) -> &'static str;

    /// Retrieve all values for the enum.
    fn get_possible_enum_string_values(&self) -> &'static [&'static str];
}

impl<T: DynamicTagData + DynamicEnumImpl> DynamicEnum for T {
    fn set_enum_string_value(&mut self, value: &str) -> RinghopperResult<()> {
        if let Some(n) = Self::from_str(value) {
            *self = n;
            Ok(())
        }
        else {
            Err(Error::InvalidEnum)
        }
    }

    fn get_enum_string_value(&self) -> &'static str {
        self.to_str()
    }

    fn get_possible_enum_string_values(&self) -> &'static [&'static str] {
        Self::str_vals()
    }
}
