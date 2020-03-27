//! Extended utilities for working with files and filesystems in Rust.

// Only allow libtest features on nightly, where they are accessible.
#![cfg_attr(all(nightly, test), feature(test))]

#[cfg(windows)]
extern crate winapi;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as sys;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as sys;

use std::fs::File;
use std::io::{Error, Result};
use std::path::Path;

/// Extension trait for `std::fs::File` which provides allocation, duplication and locking methods.
///
/// ## Notes on File Locks
///
/// This library provides whole-file locks in both shared (read) and exclusive
/// (read-write) varieties.
///
/// File locks are a cross-platform hazard since the file lock APIs exposed by
/// operating system kernels vary in subtle and not-so-subtle ways.
///
/// The API exposed by this library can be safely used across platforms as long
/// as the following rules are followed:
///
///   * Multiple locks should not be created on an individual `File` instance
///     concurrently.
///   * Duplicated files should not be locked without great care.
///   * Files to be locked should be opened with at least read or write
///     permissions.
///   * File locks may only be relied upon to be advisory.
///
/// See the tests in `lib.rs` for cross-platform lock behavior that may be
/// relied upon; see the tests in `unix.rs` and `windows.rs` for examples of
/// platform-specific behavior. File locks are implemented with
/// [`flock(2)`](http://man7.org/linux/man-pages/man2/flock.2.html) on Unix and
/// [`LockFile`](https://msdn.microsoft.com/en-us/library/windows/desktop/aa365202(v=vs.85).aspx)
/// on Windows.
pub trait FileExt {

    /// Returns a duplicate instance of the file.
    ///
    /// The returned file will share the same file position as the original
    /// file.
    ///
    /// If using rustc version 1.9 or later, prefer using `File::try_clone` to this.
    ///
    /// # Notes
    ///
    /// This is implemented with
    /// [`dup(2)`](http://man7.org/linux/man-pages/man2/dup.2.html) on Unix and
    /// [`DuplicateHandle`](https://msdn.microsoft.com/en-us/library/windows/desktop/ms724251(v=vs.85).aspx)
    /// on Windows.
    fn duplicate(&self) -> Result<File>;

    /// Returns the amount of physical space allocated for a file.
    fn allocated_size(&self) -> Result<u64>;

    /// Ensures that at least `len` bytes of disk space are allocated for the
    /// file, and the file size is at least `len` bytes. After a successful call
    /// to `allocate`, subsequent writes to the file within the specified length
    /// are guaranteed not to fail because of lack of disk space.
    fn allocate(&self, len: u64) -> Result<()>;

    /// Locks the file for shared usage, blocking if the file is currently
    /// locked exclusively.
    fn lock_shared(&self) -> Result<()>;

    /// Locks the file for exclusive usage, blocking if the file is currently
    /// locked.
    fn lock_exclusive(&self) -> Result<()>;

    /// Locks the file for shared usage, or returns a an error if the file is
    /// currently locked (see `lock_contended_error`).
    fn try_lock_shared(&self) -> Result<()>;

    /// Locks the file for exclusive usage, or returns a an error if the file is
    /// currently locked (see `lock_contended_error`).
    fn try_lock_exclusive(&self) -> Result<()>;

    /// Unlocks the file.
    fn unlock(&self) -> Result<()>;
}

impl FileExt for File {
    fn duplicate(&self) -> Result<File> {
        sys::duplicate(self)
    }
    fn allocated_size(&self) -> Result<u64> {
        sys::allocated_size(self)
    }
    fn allocate(&self, len: u64) -> Result<()> {
        sys::allocate(self, len)
    }
    fn lock_shared(&self) -> Result<()> {
        sys::lock_shared(self)
    }
    fn lock_exclusive(&self) -> Result<()> {
        sys::lock_exclusive(self)
    }
    fn try_lock_shared(&self) -> Result<()> {
        sys::try_lock_shared(self)
    }
    fn try_lock_exclusive(&self) -> Result<()> {
        sys::try_lock_exclusive(self)
    }
    fn unlock(&self) -> Result<()> {
        sys::unlock(self)
    }
}

/// Returns the error that a call to a try lock method on a contended file will
/// return.
pub fn lock_contended_error() -> Error {
    sys::lock_error()
}

/// `FsStats` contains some common stats about a file system.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FsStats {
    free_space: u64,
    available_space: u64,
    total_space: u64,
    allocation_granularity: u64,
}

impl FsStats {
    /// Returns the number of free bytes in the file system containing the provided
    /// path.
    pub fn free_space(&self) -> u64 {
        self.free_space
    }

    /// Returns the available space in bytes to non-priveleged users in the file
    /// system containing the provided path.
    pub fn available_space(&self) -> u64 {
        self.available_space
    }

    /// Returns the total space in bytes in the file system containing the provided
    /// path.
    pub fn total_space(&self) -> u64 {
        self.total_space
    }

    /// Returns the filesystem's disk space allocation granularity in bytes.
    /// The provided path may be for any file in the filesystem.
    ///
    /// On Posix, this is equivalent to the filesystem's block size.
    /// On Windows, this is equivalent to the filesystem's cluster size.
    pub fn allocation_granularity(&self) -> u64 {
        self.allocation_granularity
    }
}

/// Get the stats of the file system containing the provided path.
pub fn statvfs<P>(path: P) -> Result<FsStats> where P: AsRef<Path> {
    sys::statvfs(path.as_ref())
}

/// Returns the number of free bytes in the file system containing the provided
/// path.
pub fn free_space<P>(path: P) -> Result<u64> where P: AsRef<Path> {
    statvfs(path).map(|stat| stat.free_space)
}

/// Returns the available space in bytes to non-priveleged users in the file
/// system containing the provided path.
pub fn available_space<P>(path: P) -> Result<u64> where P: AsRef<Path> {
    statvfs(path).map(|stat| stat.available_space)
}

/// Returns the total space in bytes in the file system containing the provided
/// path.
pub fn total_space<P>(path: P) -> Result<u64> where P: AsRef<Path> {
    statvfs(path).map(|stat| stat.total_space)
}

/// Returns the filesystem's disk space allocation granularity in bytes.
/// The provided path may be for any file in the filesystem.
///
/// On Posix, this is equivalent to the filesystem's block size.
/// On Windows, this is equivalent to the filesystem's cluster size.
pub fn allocation_granularity<P>(path: P) -> Result<u64> where P: AsRef<Path> {
    statvfs(path).map(|stat| stat.allocation_granularity)
}

#[cfg(test)]
mod test;
