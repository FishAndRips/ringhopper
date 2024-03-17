use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use crc64::crc64;
use primitives::error::{Error, RinghopperResult};
use primitives::primitive::{HALO_PATH_SEPARATOR, TagGroup, TagPath};
use primitives::tag::{ParseStrictness, PrimaryTagStructDyn};

/// Tag tree implementation for traversing and loading/saving tags.
pub trait TagTree {
    /// Get a copy of the tag in the tag tree if it exists.
    ///
    /// Returns `Err` if it does not exist.
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>>;

    /// Open the tag, getting a thread-safe, potentially shared version of the tag.
    ///
    /// For tag trees that implement caching, this can return a direct reference to the in-cache version of the tag,
    /// preventing an extra copy.
    ///
    /// If this is not overridden, a copy will be returned, instead.
    fn open_tag_shared(&self, path: &TagPath) -> RinghopperResult<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.open_tag_copy(path).map(|b| Arc::new(Mutex::new(b)))
    }

    /// Get all files in the path.
    ///
    /// Returns `None` if the path does not exist.
    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>>;

    /// Write the tag into the tag tree.
    ///
    /// Returns `true` if the tag was actually saved. Errors on failure or if the tag tree is read-only.
    fn write_tag(&mut self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool>;

    /// Check if the tag tree is read-only.
    fn is_read_only(&self) -> bool;

    /// Check if the tag is present in the tree.
    fn contains(&self, path: &TagPath) -> bool;

    /// Get the root tag tree item.
    fn root(&self) -> TagTreeItem;

    /// Get all tags in the tree.
    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath>;
}

/// Tag filter
///
/// This allows you to match groups with wildcard expressions.
///
/// # Wildcards:
/// - `*` matches any number of characters, including no characters
/// - `?` matches any one character
///
/// # Examples:
/// - `*` matches anything as a catch-all
/// - `*.bitmap` matches any bitmap if `group` is unset (if group is set, it matches any `.bitmap.<group>`)
pub struct TagFilter {
    filter: String,
    group: Option<TagGroup>
}

impl TagFilter {
    /// Check if the given path is likely a filter.
    pub fn is_filter(path: &str) -> bool {
        path.chars().any(|c| c == '?' || c == '*')
    }

    /// Create a tag filter.
    ///
    /// If `group` is None, then the filter matches the whole path including group. Otherwise, the filter matches only
    /// the path, while the group matches the group.
    pub fn new(filter: &str, group: Option<TagGroup>) -> Self {
        let mut fixed = String::with_capacity(filter.len());
        for c in filter.chars() {
            if std::path::is_separator(c) {
                fixed.push(HALO_PATH_SEPARATOR)
            }
            else {
                fixed.push(c)
            }
        }
        Self {
            filter: fixed,
            group
        }
    }

    /// Test that the path passes the filter.
    ///
    /// # Examples
    ///
    /// ```
    /// use ringhopper::tag::tree::TagFilter;
    /// use ringhopper::primitives::primitive::{TagPath, TagGroup};
    ///
    /// let all_bitmaps = TagFilter::new("*.bitmap", None);
    /// assert!(all_bitmaps.passes(&TagPath::from_path("something.bitmap").unwrap()));
    /// assert!(!all_bitmaps.passes(&TagPath::from_path("something.weapon").unwrap()));
    ///
    /// let all_bitmaps = TagFilter::new("*", Some(TagGroup::Bitmap));
    /// assert!(all_bitmaps.passes(&TagPath::from_path("something.bitmap").unwrap()));
    ///
    /// let all_some_bitmaps = TagFilter::new("some*", Some(TagGroup::Bitmap));
    /// assert!(all_some_bitmaps.passes(&TagPath::from_path("something.bitmap").unwrap()));
    /// assert!(!all_some_bitmaps.passes(&TagPath::from_path("nothing.bitmap").unwrap()));
    /// assert!(!all_some_bitmaps.passes(&TagPath::from_path("something.weapon").unwrap()));
    ///
    /// let campaign_maps = TagFilter::new("levels\\???\\???", Some(TagGroup::Scenario));
    /// assert!(campaign_maps.passes(&TagPath::from_path("levels\\a10\\a10.scenario").unwrap()));
    /// assert!(!campaign_maps.passes(&TagPath::from_path("levels\\test\\wizard\\wizard.scenario").unwrap()));
    /// ```
    pub fn passes(&self, path: &TagPath) -> bool {
        if let Some(n) = self.group {
            if path.group() != n {
                return false
            }
            Self::filter_passes_raw(&self.filter, path.path())
        }
        else {
            Self::filter_passes_raw(&self.filter, &path.to_internal_path())
        }
    }

    fn filter_passes_raw(mut filter: &str, mut what: &str) -> bool {
        loop {
            let filter_first = filter.chars().next();
            let what_first = what.chars().next();

            if filter_first.is_none() && what_first.is_none() {
                return true
            }
            else if filter_first.is_none() || what_first.is_none() {
                return false
            }

            let filter_first = filter_first.unwrap();
            let what_first = what_first.unwrap();

            filter = &filter[1..];
            what = &what[1..];

            if filter_first == '?' || filter_first == what_first {
                continue
            }
            else if filter_first == '*' {
                while filter.chars().next() == Some('*') {
                    filter = &filter[1..]
                }
                if filter.is_empty() {
                    return true
                }
                while !what.is_empty() {
                    if Self::filter_passes_raw(filter, what) {
                        return true
                    }
                    what = &what[1..];
                }
                return false
            }
            else {
                return false
            }
        }
    }
}

pub struct TagTreeTagIterator<'a, 'b> {
    stack: Vec<VecDeque<TagTreeItem<'a>>>,
    filter: Option<&'b TagFilter>
}

pub fn iterate_through_all_tags<'a, 'b, T: TagTree>(what: &'a T, filter: Option<&'b TagFilter>) -> TagTreeTagIterator<'a, 'b> {
    let mut iterator = TagTreeTagIterator {
        stack: vec![],
        filter
    };

    if let Some(n) = what.root().files() {
        iterator.stack.push(n.into())
    }

    iterator
}

impl<'a, 'b> Iterator for TagTreeTagIterator<'a, 'b> {
    type Item = TagPath;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let last = self.stack.last_mut()?;
            let first = match last.pop_front() {
                Some(n) => n,
                None => {
                    self.stack.pop();
                    continue
                }
            };

            let found = match first.item_type {
                TagTreeItemType::Tag => first.tag_path().unwrap_or_else(|| panic!("found a tag in the tag tree tag iterator, but it does not have a TagPath")),
                TagTreeItemType::Directory => {
                    if let Some(n) = first.files() {
                        self.stack.push(n.into());
                    }
                    continue
                }
            };

            if let Some(n) = &self.filter {
                if !n.passes(&found) {
                    continue
                }
            }

            return Some(found)
        }
    }
}

/// Denotes an item type for identifying a [`TagTreeItem`].
#[derive(Copy, Clone, PartialEq)]
pub enum TagTreeItemType {
    /// The item represents a tag or tag file.
    ///
    /// Note that, in some cases, the validity of this being an actual tag and not just a file that happens to have a
    /// tag extension is not guaranteed.
    Tag,

    /// The item represents a directory that can be further traversed.
    Directory
}

/// Denotes a tag tree item for traversing a [`TagTree`].
#[derive(Clone)]
pub struct TagTreeItem<'a> {
    item_type: TagTreeItemType,
    path: Cow<'a, str>,
    tag_group: Option<TagGroup>,
    tag_tree: &'a dyn TagTree
}

impl<'a> TagTreeItem<'a> {
    /// Instantiates a new item for the target tag tree.
    ///
    /// # Panics
    ///
    /// Panics if tag_group is set for a directory or not set for a tag.
    pub fn new(item_type: TagTreeItemType, path: Cow<'a, str>, tag_group: Option<TagGroup>, tag_tree: &'a dyn TagTree) -> Self {
        assert!((item_type == TagTreeItemType::Tag) ^ tag_group.is_none());
        Self {
            item_type, path, tag_group, tag_tree
        }
    }

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
    pub fn files(&self) -> Option<Vec<TagTreeItem<'a>>> {
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

    /// Get the path as a string, excluding extensions if it is a tag path.
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
        Some(TagPath::new(&self.path, self.tag_group.unwrap()).unwrap())
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
    inner: T,

    // wrapped in Mutex to allow writing to state even in immutable references
    tag_cache: Mutex<HashMap<TagPath, Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>>>,
    strategy: CachingTagTreeWriteStrategy
}

impl<T: TagTree> CachingTagTree<T> {
    /// Wrap a tag tree with a cache.
    pub fn new(inner: T, strategy: CachingTagTreeWriteStrategy) -> Self {
        Self {
            inner,
            tag_cache: Mutex::new(HashMap::new()),
            strategy
        }
    }

    /// Get the inner instance as a reference.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Get the inner instance as a mutable reference.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    /// Consume the cache and return the inner value.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Get a direct reference to the tag in the cache.
    ///
    /// Returns `None` if no such tag is cached.
    pub fn get(&mut self, path: &TagPath) -> Option<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.tag_cache
            .lock()
            .unwrap()
            .get(path)
            .map(Clone::clone)
    }

    /// Evict a tag from the tag cache and return it if it existed.
    ///
    /// Returns `None` if no such tag was found in the cache.
    pub fn evict(&mut self, path: &TagPath) -> Option<Box<dyn PrimaryTagStructDyn>> {
        self.tag_cache
            .lock()
            .unwrap()
            .remove(path)
            .map(|tag| Arc::into_inner(tag).unwrap().into_inner().unwrap())
    }

    /// Write the tag to the delegate.
    ///
    /// Returns `Err(Error::TagNotFound)` if the tag is not open, or some other [`Error`] if an error occurs on the delegate.
    ///
    /// Otherwise, it forwards the result from the inner tag tree.
    pub fn commit(&mut self, path: &TagPath) -> RinghopperResult<bool> {
        let cache = self.tag_cache.lock().unwrap();
        let tag = cache.get(path).ok_or_else(|| Error::TagNotFound(path.clone()))?;
        let result = self.inner.write_tag(path, tag.as_ref().lock().unwrap().as_ref())?;
        Ok(result)
    }

    /// Write all tags to the delegate.
    ///
    /// Returns a vector of all tags that couldn't be written, with a corresponding [`Error`].
    pub fn commit_all(&mut self) -> Vec<(TagPath, Error)> {
        let cache = self.tag_cache.lock().unwrap();
        let mut errors = Vec::new();

        for (k, v) in cache.iter() {
            match self.inner.write_tag(k, v.lock().unwrap().clone_inner().as_ref()) {
                Ok(_) => (),
                Err(e) => errors.push((k.to_owned(), e))
            }
        }

        errors
    }
}

impl<T: TagTree> TagTree for CachingTagTree<T> {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        self.open_tag_shared(path)
            .map(|tag| tag.lock().unwrap().clone_inner())
    }
    fn open_tag_shared(&self, path: &TagPath) -> RinghopperResult<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        let mut cache = self.tag_cache.lock().unwrap();
        if let Some(n) = cache.get(path) {
            return Ok(n.clone())
        }
        let result = self.inner.open_tag_copy(path)?;
        let cached = Arc::new(Mutex::new(result));
        cache.insert(path.clone(), cached.clone());
        Ok(cached)
    }
    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>> {
        self.inner.files_in_path(path)
    }
    fn write_tag(&mut self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        if self.strategy == CachingTagTreeWriteStrategy::Instant {
            self.inner.write_tag(path, tag)?;
        }
        self.tag_cache.lock().unwrap().insert(path.to_owned(), Arc::new(Mutex::new(tag.clone_inner())));
        Ok(true)
    }
    fn contains(&self, path: &TagPath) -> bool {
        self.inner.contains(path)
    }
    fn root(&self) -> TagTreeItem {
        self.inner.root()
    }
    fn is_read_only(&self) -> bool {
        self.strategy == CachingTagTreeWriteStrategy::Manual || self.inner.is_read_only()
    }
    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
        self.inner.get_all_tags_with_filter(filter)
    }
}

#[derive(Clone)]
pub struct VirtualTagsDirectory {
    directories: Vec<PathBuf>,
    strictness: ParseStrictness,
    cow_output: Option<PathBuf>
}

impl VirtualTagsDirectory {
    /// Initialize a virtual tags directory.
    ///
    /// Lower directories have higher priority and are chosen first, and it is where tags will be
    /// written to by default. Tags that are unmodified will not be saved.
    ///
    /// `cow_output` is where new or modified tags will be written to.
    ///
    /// Returns `Error::InvalidTagsDirectory` if any directories passed do not exist.
    pub fn new<P: AsRef<Path>>(directories: &[P], cow_output: Option<PathBuf>) -> RinghopperResult<Self> {
        let directories: Vec<PathBuf> = directories.iter().map(|path| path.as_ref().to_path_buf()).collect();

        for i in &directories {
            if !i.is_dir() {
                return Err(Error::InvalidTagsDirectory)
            }
        }

        Ok(Self { directories, strictness: ParseStrictness::Strict, cow_output })
    }

    /// Set the strictness for opening tags.
    pub fn set_strictness(&mut self, strictness: ParseStrictness) -> () {
        self.strictness = strictness
    }

    /// Write the tag to the desired tags directory.
    ///
    /// Note that if there is a cow, `directory` will be ignored and the cow will be used instead.
    pub fn write_tag_to_directory(&self, path: &TagPath, tag: &dyn PrimaryTagStructDyn, directory: usize) -> RinghopperResult<bool> {
        let tag_file = tag.to_tag_file()?;
        let hash = Self::hash_file(tag_file.as_slice());

        let path_for_tag = match self.path_for_tag(path).map(|(_, path)| path) {
            Some(n) => {
                let original_hash = std::fs::read(&n)
                    .map(|f| Self::hash_file(f.as_slice()))
                    .map_err(|e| Error::FailedToReadFile(n.clone(), e))?;

                if original_hash == hash {
                    return Ok(false)
                }

                n
            },
            None => match &self.cow_output {
                Some(_) => PathBuf::new(), // will be overwritten in file_to_write_to
                None => self.directories[directory].join(path.to_native_path())
            }
        };

        let file_to_write_to = match &self.cow_output {
            Some(n) => n.join(path.to_native_path()),
            None => path_for_tag
        };

        let parent = file_to_write_to.parent().unwrap();
        std::fs::create_dir_all(&parent).map_err(|e| Error::FailedToWriteFile(parent.to_path_buf(), e))?;
        std::fs::write(&file_to_write_to, tag_file).map_err(|e| Error::FailedToWriteFile(file_to_write_to, e))?;

        Ok(true)
    }

    /// Get the directory index and path the tag is located in.
    pub fn path_for_tag(&self, path: &TagPath) -> Option<(usize, PathBuf)> {
        let native_path = path.to_native_path();
        for index in 0..self.directories.len() {
            let directory = &self.directories[index];
            let path_to_test = directory.join(&native_path);
            if !path_to_test.exists() {
                continue
            }
            return Some((index, path_to_test));
        }
        None
    }

    fn hash_file(file: &[u8]) -> u64 {
        crc64(u64::MAX, file)
    }
}

impl TagTree for VirtualTagsDirectory {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        let file_path = self.path_for_tag(path).ok_or_else(|| Error::TagNotFound(path.clone()))?.1;
        let file = std::fs::read(&file_path).map_err(|e| Error::FailedToReadFile(file_path, e))?;
        let hash = Self::hash_file(file.as_slice());
        let mut tag = ringhopper_structs::read_any_tag_from_file_buffer(&file, self.strictness)
            .map_err(|e| match e {
                Error::TagParseFailure(_) => Error::CorruptedTag(path.clone(), vec![e]),
                e => e
            })?;
        tag.set_hash(hash);
        Ok(tag)
    }
    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>> {
        let mut result = Vec::new();
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

                    let mut path = path.strip_prefix(dir_path).unwrap().to_owned().into_os_string().into_string().ok()?;
                    if !is_dir {
                        let extension = path.rfind('.')?;
                        let tag_group = TagGroup::from_str(&path[extension + 1..]).ok()?;
                        path.truncate(extension);
                        Some(TagTreeItem::new(
                            TagTreeItemType::Tag,
                            Cow::Owned(path),
                            Some(tag_group),
                            self
                        ))
                    }

                    else {
                        Some(TagTreeItem::new(
                            TagTreeItemType::Directory,
                            Cow::Owned(path),
                            None,
                            self
                        ))
                    }
                });

            for f in entries {
                result.push(f);
            }
        }

        result.dedup_by(|a, b| {
            a.item_type == b.item_type && a.tag_group == b.tag_group && a.path == b.path
        });

        if !success {
            return None
        }

        Some(result)
    }

    fn write_tag(&mut self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        self.write_tag_to_directory(path, tag, self.path_for_tag(path).map(|(index, _)| index).unwrap_or(0))
    }

    fn is_read_only(&self) -> bool {
        false
    }

    fn contains(&self, path: &TagPath) -> bool {
        self.path_for_tag(&path).is_some()
    }

    fn root(&self) -> TagTreeItem {
        TagTreeItem::new(TagTreeItemType::Directory, Cow::default(), None, self)
    }

    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
        iterate_through_all_tags(self, filter).collect()
    }
}

/// Thread-safe wrapper for tag trees.
///
/// This internally uses an `Arc`, so cloning this tag tree actually clones a reference.
///
/// The `files_in_path` function is unavailable, as is `iterate_through_all_tags`.
///
/// Use `get_all_tags_with_filter` to get all tags in the tree.
pub struct AtomicTagTree<T: TagTree + Send> {
    inner: Arc<Mutex<T>>
}

impl<T: TagTree + Send> TagTree for AtomicTagTree<T> {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        self.inner.lock().unwrap().open_tag_copy(path)
    }

    fn open_tag_shared(&self, path: &TagPath) -> RinghopperResult<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.inner.lock().unwrap().open_tag_shared(path)
    }

    fn files_in_path(&self, _path: &str) -> Option<Vec<TagTreeItem>> {
        unimplemented!("files_in_path not implemented for AtomicTagTree")
    }

    fn write_tag(&mut self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        self.inner.lock().unwrap().write_tag(path, tag)
    }

    fn is_read_only(&self) -> bool {
        self.inner.lock().unwrap().is_read_only()
    }

    fn contains(&self, path: &TagPath) -> bool {
        self.inner.lock().unwrap().contains(path)
    }

    fn root(&self) -> TagTreeItem {
        unimplemented!("root not implemented for AtomicTagTree")
    }

    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
        self.inner.lock().unwrap().get_all_tags_with_filter(filter)
    }
}

impl<T: TagTree> TagTree for Arc<T> {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        self.as_ref().open_tag_copy(path)
    }

    fn open_tag_shared(&self, path: &TagPath) -> RinghopperResult<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.as_ref().open_tag_shared(path)
    }

    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>> {
        self.as_ref().files_in_path(path)
    }

    fn write_tag(&mut self, _path: &TagPath, _tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        unimplemented!("Arc<T: TagTree> is immutable")
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn contains(&self, path: &TagPath) -> bool {
        self.as_ref().contains(path)
    }

    fn root(&self) -> TagTreeItem {
        self.as_ref().root()
    }

    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
        self.as_ref().get_all_tags_with_filter(filter)
    }
}

impl<T: TagTree + Send> Clone for AtomicTagTree<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone()
        }
    }
}

impl<T: TagTree + Send> AtomicTagTree<T> {
    /// Instantiate a new AtomicTagTree instance.
    pub fn new(tree: T) -> Self {
        Self { inner: Arc::new(Mutex::new(tree)) }
    }

    /// Decompose the atomic tag tree into its inner tree.
    pub fn into_inner(self) -> T {
        Arc::into_inner(self.inner).unwrap().into_inner().unwrap()
    }

    /// Return a clone of the inner tag tree.
    pub fn clone_inner(&self) -> T where T: Clone {
        self.inner.lock().unwrap().clone()
    }
}

impl TagTree for Box<dyn TagTree + Send + Sync> {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        self.as_ref().open_tag_copy(path)
    }

    fn open_tag_shared(&self, path: &TagPath) -> RinghopperResult<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.as_ref().open_tag_shared(path)
    }

    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>> {
        self.as_ref().files_in_path(path)
    }

    fn write_tag(&mut self, path: &TagPath, tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        self.as_mut().write_tag(path, tag)
    }

    fn is_read_only(&self) -> bool {
        self.as_ref().is_read_only()
    }

    fn contains(&self, path: &TagPath) -> bool {
        self.as_ref().contains(path)
    }

    fn root(&self) -> TagTreeItem {
        self.as_ref().root()
    }

    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
        self.as_ref().get_all_tags_with_filter(filter)
    }
}

impl TagTree for Arc<dyn TagTree + Send + Sync> {
    fn open_tag_copy(&self, path: &TagPath) -> RinghopperResult<Box<dyn PrimaryTagStructDyn>> {
        self.as_ref().open_tag_copy(path)
    }

    fn open_tag_shared(&self, path: &TagPath) -> RinghopperResult<Arc<Mutex<Box<dyn PrimaryTagStructDyn>>>> {
        self.as_ref().open_tag_shared(path)
    }

    fn files_in_path(&self, path: &str) -> Option<Vec<TagTreeItem>> {
        self.as_ref().files_in_path(path)
    }

    fn write_tag(&mut self, _path: &TagPath, _tag: &dyn PrimaryTagStructDyn) -> RinghopperResult<bool> {
        unimplemented!("Arc<dyn TagTree + Send + Sync> is immutable")
    }

    fn is_read_only(&self) -> bool {
        true
    }

    fn contains(&self, path: &TagPath) -> bool {
        self.as_ref().contains(path)
    }

    fn root(&self) -> TagTreeItem {
        self.as_ref().root()
    }

    fn get_all_tags_with_filter(&self, filter: Option<&TagFilter>) -> Vec<TagPath> {
        self.as_ref().get_all_tags_with_filter(filter)
    }
}

#[cfg(test)]
mod test;

#[cfg(test)]
pub(crate) use self::test::MockTagTree;
