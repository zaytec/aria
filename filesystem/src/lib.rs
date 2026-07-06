//! A synchronous in-memory virtual file system for ARIA.

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

/// Errors produced by virtual file system operations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FileSystemError {
    InvalidPath,
    NotFound,
    AlreadyExists,
    NotAFile,
    NotADirectory,
    DirectoryNotEmpty,
    CannotModifyRoot,
    InvalidMove,
}

impl fmt::Display for FileSystemError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::InvalidPath => "invalid absolute path",
            Self::NotFound => "path not found",
            Self::AlreadyExists => "path already exists",
            Self::NotAFile => "path is not a file",
            Self::NotADirectory => "path is not a directory",
            Self::DirectoryNotEmpty => "directory is not empty",
            Self::CannotModifyRoot => "cannot modify the root directory",
            Self::InvalidMove => "cannot move a directory inside itself",
        };

        formatter.write_str(message)
    }
}

impl Error for FileSystemError {}

/// A normalized absolute path in the virtual file system.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Path {
    components: Vec<String>,
}

impl Path {
    /// Parses and normalizes an absolute path.
    pub fn new(value: impl AsRef<str>) -> Result<Self, FileSystemError> {
        let value = value.as_ref();
        if !value.starts_with('/') || value.contains('\0') {
            return Err(FileSystemError::InvalidPath);
        }

        let mut components = Vec::new();
        for component in value.split('/') {
            match component {
                "" | "." => {}
                ".." => {
                    if components.pop().is_none() {
                        return Err(FileSystemError::InvalidPath);
                    }
                }
                component => components.push(component.to_owned()),
            }
        }

        Ok(Self { components })
    }

    /// Returns the root path.
    pub fn root() -> Self {
        Self {
            components: Vec::new(),
        }
    }

    /// Returns whether this path refers to the root directory.
    pub fn is_root(&self) -> bool {
        self.components.is_empty()
    }

    fn parent_components(&self) -> Result<(&[String], &str), FileSystemError> {
        let Some((name, parent)) = self.components.split_last() else {
            return Err(FileSystemError::CannotModifyRoot);
        };

        Ok((parent, name))
    }

    fn with_name(&self, name: &str) -> Result<Self, FileSystemError> {
        validate_name(name)?;
        let (parent, _) = self.parent_components()?;
        let mut components = parent.to_vec();
        components.push(name.to_owned());
        Ok(Self { components })
    }
}

impl fmt::Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_root() {
            return formatter.write_str("/");
        }

        write!(formatter, "/{}", self.components.join("/"))
    }
}

impl TryFrom<&str> for Path {
    type Error = FileSystemError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

/// The kind of a file system entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileType {
    File,
    Directory,
}

/// Metadata describing a file system entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    name: String,
    file_type: FileType,
    size: usize,
}

impl FileMetadata {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn file_type(&self) -> FileType {
        self.file_type
    }

    /// Returns bytes for files and direct child count for directories.
    pub fn size(&self) -> usize {
        self.size
    }
}

/// A text file stored in the virtual file system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct File {
    name: String,
    contents: String,
}

impl File {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn contents(&self) -> &str {
        &self.contents
    }

    pub fn metadata(&self) -> FileMetadata {
        FileMetadata {
            name: self.name.clone(),
            file_type: FileType::File,
            size: self.contents.len(),
        }
    }
}

/// A directory stored in the virtual file system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Directory {
    name: String,
    entries: BTreeMap<String, Node>,
}

impl Directory {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn metadata(&self) -> FileMetadata {
        FileMetadata {
            name: self.name.clone(),
            file_type: FileType::Directory,
            size: self.entries.len(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum Node {
    File(File),
    Directory(Directory),
}

impl Node {
    fn metadata(&self) -> FileMetadata {
        match self {
            Self::File(file) => file.metadata(),
            Self::Directory(directory) => directory.metadata(),
        }
    }

    fn set_name(&mut self, name: String) {
        match self {
            Self::File(file) => file.name = name,
            Self::Directory(directory) => directory.name = name,
        }
    }
}

/// A synchronous hierarchical in-memory file system.
pub struct FileSystem {
    root: Directory,
}

impl FileSystem {
    pub fn new() -> Self {
        Self {
            root: Directory {
                name: "/".to_owned(),
                entries: BTreeMap::new(),
            },
        }
    }

    pub fn create_file(&mut self, path: &Path) -> Result<(), FileSystemError> {
        let (parent, name) = path.parent_components()?;
        let directory = directory_mut(&mut self.root, parent)?;
        if directory.entries.contains_key(name) {
            return Err(FileSystemError::AlreadyExists);
        }

        directory.entries.insert(
            name.to_owned(),
            Node::File(File {
                name: name.to_owned(),
                contents: String::new(),
            }),
        );
        Ok(())
    }

    pub fn read_file<'a>(&'a self, path: &Path) -> Result<&'a str, FileSystemError> {
        match self.node(path)? {
            Node::File(file) => Ok(file.contents()),
            Node::Directory(_) => Err(FileSystemError::NotAFile),
        }
    }

    pub fn write_file(
        &mut self,
        path: &Path,
        contents: impl Into<String>,
    ) -> Result<(), FileSystemError> {
        match self.node_mut(path)? {
            Node::File(file) => {
                file.contents = contents.into();
                Ok(())
            }
            Node::Directory(_) => Err(FileSystemError::NotAFile),
        }
    }

    pub fn delete_file(&mut self, path: &Path) -> Result<(), FileSystemError> {
        let (parent, name) = path.parent_components()?;
        let directory = directory_mut(&mut self.root, parent)?;
        match directory.entries.get(name) {
            Some(Node::File(_)) => {}
            Some(Node::Directory(_)) => return Err(FileSystemError::NotAFile),
            None => return Err(FileSystemError::NotFound),
        }

        directory.entries.remove(name);
        Ok(())
    }

    pub fn create_directory(&mut self, path: &Path) -> Result<(), FileSystemError> {
        let (parent, name) = path.parent_components()?;
        let directory = directory_mut(&mut self.root, parent)?;
        if directory.entries.contains_key(name) {
            return Err(FileSystemError::AlreadyExists);
        }

        directory.entries.insert(
            name.to_owned(),
            Node::Directory(Directory {
                name: name.to_owned(),
                entries: BTreeMap::new(),
            }),
        );
        Ok(())
    }

    pub fn remove_directory(&mut self, path: &Path) -> Result<(), FileSystemError> {
        let (parent, name) = path.parent_components()?;
        let directory = directory_mut(&mut self.root, parent)?;
        match directory.entries.get(name) {
            Some(Node::Directory(child)) if child.entries.is_empty() => {}
            Some(Node::Directory(_)) => return Err(FileSystemError::DirectoryNotEmpty),
            Some(Node::File(_)) => return Err(FileSystemError::NotADirectory),
            None => return Err(FileSystemError::NotFound),
        }

        directory.entries.remove(name);
        Ok(())
    }

    pub fn exists(&self, path: &Path) -> bool {
        path.is_root() || self.node(path).is_ok()
    }

    pub fn list_directory(&self, path: &Path) -> Result<Vec<FileMetadata>, FileSystemError> {
        let directory = self.directory(path)?;
        Ok(directory.entries.values().map(Node::metadata).collect())
    }

    pub fn rename(&mut self, path: &Path, new_name: &str) -> Result<(), FileSystemError> {
        let destination = path.with_name(new_name)?;
        self.r#move(path, &destination)
    }

    pub fn r#move(&mut self, source: &Path, destination: &Path) -> Result<(), FileSystemError> {
        self.validate_transfer(source, destination, true)?;
        let mut node = self.remove_node(source)?;
        let (_, name) = destination.parent_components()?;
        node.set_name(name.to_owned());
        self.insert_node(destination, node)
    }

    pub fn copy(&mut self, source: &Path, destination: &Path) -> Result<(), FileSystemError> {
        self.validate_transfer(source, destination, false)?;
        let mut node = self.node(source)?.clone();
        let (_, name) = destination.parent_components()?;
        node.set_name(name.to_owned());
        self.insert_node(destination, node)
    }

    fn node(&self, path: &Path) -> Result<&Node, FileSystemError> {
        if path.is_root() {
            return Err(FileSystemError::NotFound);
        }

        let (name, parent) = path.components.split_last().unwrap();
        directory(&self.root, parent)?
            .entries
            .get(name)
            .ok_or(FileSystemError::NotFound)
    }

    fn node_mut(&mut self, path: &Path) -> Result<&mut Node, FileSystemError> {
        let (parent, name) = path.parent_components()?;
        directory_mut(&mut self.root, parent)?
            .entries
            .get_mut(name)
            .ok_or(FileSystemError::NotFound)
    }

    fn directory(&self, path: &Path) -> Result<&Directory, FileSystemError> {
        directory(&self.root, &path.components)
    }

    fn validate_transfer(
        &self,
        source: &Path,
        destination: &Path,
        moving: bool,
    ) -> Result<(), FileSystemError> {
        if source.is_root() || destination.is_root() {
            return Err(FileSystemError::CannotModifyRoot);
        }

        let source_node = self.node(source)?;
        if self.exists(destination) {
            return Err(FileSystemError::AlreadyExists);
        }

        let (destination_parent, _) = destination.parent_components()?;
        directory(&self.root, destination_parent)?;

        if moving
            && matches!(source_node, Node::Directory(_))
            && destination.components.starts_with(&source.components)
        {
            return Err(FileSystemError::InvalidMove);
        }

        Ok(())
    }

    fn remove_node(&mut self, path: &Path) -> Result<Node, FileSystemError> {
        let (parent, name) = path.parent_components()?;
        directory_mut(&mut self.root, parent)?
            .entries
            .remove(name)
            .ok_or(FileSystemError::NotFound)
    }

    fn insert_node(&mut self, path: &Path, node: Node) -> Result<(), FileSystemError> {
        let (parent, name) = path.parent_components()?;
        directory_mut(&mut self.root, parent)?
            .entries
            .insert(name.to_owned(), node);
        Ok(())
    }
}

impl Default for FileSystem {
    fn default() -> Self {
        Self::new()
    }
}

fn directory<'a>(
    root: &'a Directory,
    components: &[String],
) -> Result<&'a Directory, FileSystemError> {
    let mut current = root;
    for component in components {
        current = match current.entries.get(component) {
            Some(Node::Directory(directory)) => directory,
            Some(Node::File(_)) => return Err(FileSystemError::NotADirectory),
            None => return Err(FileSystemError::NotFound),
        };
    }
    Ok(current)
}

fn directory_mut<'a>(
    root: &'a mut Directory,
    components: &[String],
) -> Result<&'a mut Directory, FileSystemError> {
    let mut current = root;
    for component in components {
        current = match current.entries.get_mut(component) {
            Some(Node::Directory(directory)) => directory,
            Some(Node::File(_)) => return Err(FileSystemError::NotADirectory),
            None => return Err(FileSystemError::NotFound),
        };
    }
    Ok(current)
}

fn validate_name(name: &str) -> Result<(), FileSystemError> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\0') {
        return Err(FileSystemError::InvalidPath);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{FileSystem, FileSystemError, FileType, Path};

    fn path(value: &str) -> Path {
        Path::new(value).unwrap()
    }

    #[test]
    fn requires_and_normalizes_absolute_paths() {
        assert_eq!(Path::new("relative"), Err(FileSystemError::InvalidPath));
        assert_eq!(
            path("/projects/./aria/../demo").to_string(),
            "/projects/demo"
        );
        assert_eq!(Path::new("/../outside"), Err(FileSystemError::InvalidPath));
    }

    #[test]
    fn creates_reads_writes_and_deletes_nested_files() {
        let mut filesystem = FileSystem::new();
        filesystem.create_directory(&path("/projects")).unwrap();
        filesystem
            .create_directory(&path("/projects/aria"))
            .unwrap();
        filesystem
            .create_file(&path("/projects/aria/session.txt"))
            .unwrap();
        filesystem
            .write_file(&path("/projects/aria/session.txt"), "hello ARIA")
            .unwrap();

        assert_eq!(
            filesystem.read_file(&path("/projects/aria/session.txt")),
            Ok("hello ARIA")
        );

        filesystem
            .delete_file(&path("/projects/aria/session.txt"))
            .unwrap();
        assert!(!filesystem.exists(&path("/projects/aria/session.txt")));
    }

    #[test]
    fn lists_directories_in_name_order() {
        let mut filesystem = FileSystem::new();
        filesystem.create_directory(&path("/projects")).unwrap();
        filesystem.create_file(&path("/zeta.txt")).unwrap();

        let entries = filesystem.list_directory(&Path::root()).unwrap();

        assert_eq!(entries[0].name(), "projects");
        assert_eq!(entries[0].file_type(), FileType::Directory);
        assert_eq!(entries[1].name(), "zeta.txt");
        assert_eq!(entries[1].file_type(), FileType::File);
    }

    #[test]
    fn only_removes_empty_directories() {
        let mut filesystem = FileSystem::new();
        filesystem.create_directory(&path("/projects")).unwrap();
        filesystem.create_file(&path("/projects/aria.txt")).unwrap();

        assert_eq!(
            filesystem.remove_directory(&path("/projects")),
            Err(FileSystemError::DirectoryNotEmpty)
        );

        filesystem.delete_file(&path("/projects/aria.txt")).unwrap();
        filesystem.remove_directory(&path("/projects")).unwrap();
        assert!(!filesystem.exists(&path("/projects")));
    }

    #[test]
    fn renames_entries() {
        let mut filesystem = FileSystem::new();
        filesystem.create_file(&path("/draft.txt")).unwrap();
        filesystem
            .write_file(&path("/draft.txt"), "content")
            .unwrap();

        filesystem.rename(&path("/draft.txt"), "final.txt").unwrap();

        assert!(!filesystem.exists(&path("/draft.txt")));
        assert_eq!(filesystem.read_file(&path("/final.txt")), Ok("content"));
    }

    #[test]
    fn moves_directory_trees() {
        let mut filesystem = FileSystem::new();
        filesystem.create_directory(&path("/source")).unwrap();
        filesystem.create_file(&path("/source/file.txt")).unwrap();
        filesystem.create_directory(&path("/archive")).unwrap();

        filesystem
            .r#move(&path("/source"), &path("/archive/source"))
            .unwrap();

        assert!(!filesystem.exists(&path("/source")));
        assert!(filesystem.exists(&path("/archive/source/file.txt")));
        assert_eq!(
            filesystem.r#move(&path("/archive/source"), &path("/archive/source/nested")),
            Err(FileSystemError::InvalidMove)
        );
    }

    #[test]
    fn copies_independent_directory_trees() {
        let mut filesystem = FileSystem::new();
        filesystem.create_directory(&path("/source")).unwrap();
        filesystem.create_file(&path("/source/file.txt")).unwrap();
        filesystem
            .write_file(&path("/source/file.txt"), "original")
            .unwrap();

        filesystem.copy(&path("/source"), &path("/copy")).unwrap();
        filesystem
            .write_file(&path("/source/file.txt"), "changed")
            .unwrap();

        assert_eq!(
            filesystem.read_file(&path("/copy/file.txt")),
            Ok("original")
        );
        assert_eq!(
            filesystem.read_file(&path("/source/file.txt")),
            Ok("changed")
        );
    }
}
