use std::fmt::{Display, Formatter};
use definitions::{TagCollection, UIWidgetCollection, TagCollectionTag};
use primitives::primitive::{TagPath, TagReference};

#[derive(Debug)]
pub enum TagCollectionError {
    InvalidTextFile,
    BadTagPath(usize, String)
}

impl Display for TagCollectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidTextFile => f.write_str("invalid text file (was not utf-8)"),
            Self::BadTagPath(line, s) => f.write_fmt(format_args!("[line {line}] `{s}` does not correspond to a valid path"))
        }
    }
}

pub trait TagCollectionFunctions {
    /// Make a tag from the list file data.
    ///
    /// The data must be UTF-8 and must be a list of valid tag paths and/or empty lines.
    fn from_text_data(text_data: &[u8]) -> Result<Self, TagCollectionError> where Self: Sized;
}

macro_rules! make_tag_collection_parser {
    ($s:ty) => {
        impl TagCollectionFunctions for $s {
            fn from_text_data(text_data: &[u8]) -> Result<Self, TagCollectionError> {
                let text_file = std::str::from_utf8(text_data).map_err(|_| TagCollectionError::InvalidTextFile)?;
                let mut tag_collection = Self::default();

                let mut line = 0;
                for i in text_file.lines() {
                    line += 1;
                    if i.is_empty() {
                        continue
                    }
                    tag_collection.tags.items.push(TagCollectionTag {
                        reference: TagReference::Set(
                            TagPath::from_path(i).map_err(|_| TagCollectionError::BadTagPath(line, i.to_owned()))?
                        )
                    });
                }

                Ok(tag_collection)
            }
        }
    };
}

make_tag_collection_parser!(TagCollection);
make_tag_collection_parser!(UIWidgetCollection);
