use std::collections::{HashMap, HashSet};
use std::fs::{read, write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{TagGroup, TagPath};
use primitives::tag::{ParseStrictness, PrimaryTagStructDyn};

/// Tag tree implementation for traversing and loading/saving tags.
pub trait TagTree {
    /// Get the tag in the tag tree if it exists.
    ///
    /// Returns `Err` if it does not exist.
    fn get_tag(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>>;

    /// Get all files in the path.
    ///
    /// Returns `None` if the path does not exist.
    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>>;

    /// Write the tag into the tag tree.
    fn write_tag(&self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<()>;

    /// Get the root tag tree item.
    fn root(&self) -> TagTreeItem where Self: Sized {
        TagTreeItem {
            tag_tree: self,
            path: String::new(),
            item_type: TagTreeItemType::Directory,
            tag_group: None
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TagTreeItemType {
    Tag,
    Directory
}

#[derive(Clone)]
pub struct TagTreeItem<'a> {
    item_type: TagTreeItemType,
    tag_tree: &'a dyn TagTree,
    tag_group: Option<TagGroup>,
    path: String
}

impl<'a> TagTreeItem<'a> {
    /// Get the type of item this is.
    pub fn item_type(&self) -> TagTreeItemType {
        self.item_type
    }

    /// Return `true` if this is a tag.
    pub fn is_tag(&self) -> bool {
        self.item_type == TagTreeItemType::Tag
    }

    /// Return `true` if this is a directory.
    pub fn is_directory(&self) -> bool {
        self.item_type == TagTreeItemType::Directory
    }

    /// Get the inner files of this directory.
    ///
    /// Returns `None` if this is not a directory or it no longer exists.
    pub fn files(&self) -> Option<Vec<TagTreeItem>> {
        if self.item_type == TagTreeItemType::Directory {
            self.tag_tree.files_in_path(&self.path)
        }
        else {
            None
        }
    }

    /// Get the tag group if this is a tag.
    ///
    /// Returns `None` if this is not a tag.
    pub fn tag_group(&self) -> Option<TagGroup> {
        self.tag_group
    }

    /// Get the path as a string, including any extensions.
    pub fn path_str(&self) -> &str {
        &self.path
    }

    /// Get this as a tag path, if it is a tag.
    ///
    /// Returns `None` if this is not a tag.
    pub fn tag_path(&self) -> Option<TagPath> {
        if self.is_directory() {
            return None
        }
        Some(TagPath::from_path(self.path_str()).unwrap())
    }
}

#[derive(PartialEq)]
pub enum CachingTagTreeWriteStrategy {
    /// Writing a tag instantly commits it into the file system.
    Instant,

    /// Writing a tag only impacts what is cached and does not actually write to the delegate.
    Manual
}

pub struct CachingTagTree<T> where T: TagTree {
    delegate: T,
    tag_cache: Arc<Mutex<HashMap<TagPath, Box<dyn PrimaryTagStructDyn>>>>,
    strategy: CachingTagTreeWriteStrategy
}

impl<T: TagTree + Sized> CachingTagTree<T> {
    /// Wrap a tag tree with a cache.
    pub fn new(delegate: T, strategy: CachingTagTreeWriteStrategy) -> Self {
        Self {
            delegate,
            tag_cache: Arc::new(Mutex::new(HashMap::new())),
            strategy
        }
    }

    /// Evict a tag from the tag cache.
    pub fn remove_tag(&mut self, path: TagPath) -> Option<Box<dyn PrimaryTagStructDyn>> {
        self.tag_cache.lock().unwrap().remove(&path)
    }

    /// Write the tag to the delegate.
    ///
    /// Returns `Err(Error::FileNotFound)` if the tag is not open, or some other [`Error`] if an error occurs on the delegate.
    pub fn commit(&self, path: &TagPath) -> RinghopperResult<()> {
        let cache = self.tag_cache.lock().unwrap();
        let tag = cache.get(path).ok_or(Error::FileNotFound)?;
        self.delegate.write_tag(path, tag.as_ref())
    }

    /// Write all tags to the delegate.
    pub fn commit_all(&self) -> Vec<(TagPath, Error)> {
        self.tag_cache
            .lock()
            .unwrap()
            .iter()
            .filter_map(|f| {
                let error = self.delegate
                    .write_tag(f.0, f.1.as_ref())
                    .err()?;
                Some((f.0.to_owned(), error))
            })
            .collect()
    }
}

impl<T: TagTree> TagTree for CachingTagTree<T> {
    fn get_tag(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        let mut cache = self.tag_cache.lock().unwrap();
        if let Some(n) = cache.get(path) {
            return Ok(n.clone_inner())
        }
        let result = self.delegate.get_tag(path)?;
        cache.insert(path.clone(), result.clone_inner());
        Ok(result)
    }
    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>> {
        self.delegate.files_in_path(path)
    }
    fn write_tag(&self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<()> {
        if self.strategy == CachingTagTreeWriteStrategy::Instant {
            self.delegate.write_tag(path, tag)?;
        }
        self.tag_cache.lock().unwrap().insert(path.to_owned(), tag.clone_inner());
        Ok(())
    }
}

pub struct VirtualTagDirectory {
    directories: Vec<PathBuf>
}

impl VirtualTagDirectory {
    /// Initialize a virtual tags directory.
    ///
    /// Lower directories have higher priority and are chosen first, and it is where tags will be
    /// written to by default.
    ///
    /// Returns `Error::InvalidTagsDirectory` if any directories passed do not exist.
    pub fn new<P: AsRef<Path>>(directories: &[P]) -> RinghopperResult<Self> {
        let directories: Vec<PathBuf> = directories.iter().map(|path| path.as_ref().to_path_buf()).collect();

        for i in &directories {
            if !i.is_dir() {
                return Err(Error::InvalidTagsDirectory)
            }
        }

        Ok(Self { directories })
    }

    fn path_for_tag(&self, path: &TagPath) -> Option<PathBuf> {
        let native_path = path.to_native_path();
        for index in 0..self.directories.len() {
            let directory = &self.directories[index];
            let path_to_test = directory.join(&native_path);
            if !path_to_test.exists() {
                continue
            }
            return Some(path_to_test);
        }
        None
    }
}

impl TagTree for VirtualTagDirectory {
    fn get_tag(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        let path = self.path_for_tag(path).ok_or(Error::FileNotFound)?;
        let file = read(path).map_err(|_| Error::FailedToReadFile)?;
        return ringhopper_structs::read_any_tag_from_file_buffer(&file, ParseStrictness::Strict)
    }
    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>> {
        let mut result = Vec::new();
        let mut items_found = HashSet::new();
        let mut success = false;

        for dir_path in &self.directories {
            let dir = match std::fs::read_dir(dir_path.join(path)) {
                Ok(n) => n,
                Err(_) => continue
            };
            success = true;

            let entries = dir
                .filter_map(|f| f.ok())
                .filter_map(|f| {
                    let path = f.path();
                    let is_dir = path.is_dir();
                    if !is_dir && !path.is_file() {
                        return None
                    }

                    let path = path.strip_prefix(dir_path).unwrap().to_owned();
                    if !is_dir {
                        let tag_group = TagGroup::from_str(path.extension()?.to_str()?).ok()?;

                        Some(TagTreeItem {
                            item_type: TagTreeItemType::Tag,
                            tag_tree: self,
                            path: path.into_os_string().into_string().ok()?,
                            tag_group: Some(tag_group)
                        })
                    }

                    else {
                        Some(TagTreeItem {
                            item_type: TagTreeItemType::Directory,
                            tag_tree: self,
                            path: path.into_os_string().into_string().ok()?,
                            tag_group: None
                        })
                    }
                });

            for f in entries {
                let path_base = Path::new(&f.path);
                let file_name = path_base.file_name().unwrap();
                if items_found.contains(file_name) {
                    continue
                }
                items_found.insert(file_name.to_owned());
                result.push(f);
            }
        }

        if !success {
            return None
        }

        Some(result)
    }
    fn write_tag(&self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<()> {
        let file_to_write_to = self.path_for_tag(path).unwrap_or_else(|| self.directories[0].join(path.to_native_path()));
        std::fs::create_dir_all(file_to_write_to.parent().unwrap()).map_err(|_| Error::FailedToWriteFile)?;
        write(file_to_write_to, tag.to_tag_file()?).map_err(|_| Error::FailedToReadFile)
    }
}

#[cfg(test)]
mod test;
