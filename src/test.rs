extern crate tempdir;

use crate::*;
use std::fs;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub(crate) fn tmpdir() -> tempdir::TempDir {
    tempdir::TempDir::new("fs3").unwrap()
}

pub(crate) fn tmpfile() -> (tempdir::TempDir, PathBuf) {
    let dir = tmpdir();
    let path = dir.path().join("file");
    (dir, path)
}

/// Tests file duplication.
#[test]
fn duplicate() {
    let (_dir, path) = tmpfile();
    let mut file1 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let mut file2 = file1.duplicate().unwrap();

    // Write into the first file and then drop it.
    file1.write_all(b"foo").unwrap();
    drop(file1);

    let mut buf = vec![];

    // Read from the second file; since the position is shared it will already be at EOF.
    file2.read_to_end(&mut buf).unwrap();
    assert_eq!(0, buf.len());

    // Rewind and read.
    file2.seek(SeekFrom::Start(0)).unwrap();
    file2.read_to_end(&mut buf).unwrap();
    assert_eq!(&buf, &b"foo");
}

/// Tests shared file lock operations.
#[test]
fn lock_shared() {
    let (_dir, path) = tmpfile();
    let file1 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let file2 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let file3 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();

    // Concurrent shared access is OK, but not shared and exclusive.
    file1.lock_shared().unwrap();
    file2.lock_shared().unwrap();
    assert_eq!(
        file3.try_lock_exclusive().unwrap_err().kind(),
        lock_contended_error().kind()
    );
    file1.unlock().unwrap();
    assert_eq!(
        file3.try_lock_exclusive().unwrap_err().kind(),
        lock_contended_error().kind()
    );

    // Once all shared file locks are dropped, an exclusive lock may be created;
    file2.unlock().unwrap();
    file3.lock_exclusive().unwrap();
}

/// Tests exclusive file lock operations.
#[test]
fn lock_exclusive() {
    let (_dir, path) = tmpfile();
    let file1 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let file2 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();

    // No other access is possible once an exclusive lock is created.
    file1.lock_exclusive().unwrap();
    assert_eq!(
        file2.try_lock_exclusive().unwrap_err().kind(),
        lock_contended_error().kind()
    );
    assert_eq!(
        file2.try_lock_shared().unwrap_err().kind(),
        lock_contended_error().kind()
    );

    // Once the exclusive lock is dropped, the second file is able to create a lock.
    file1.unlock().unwrap();
    file2.lock_exclusive().unwrap();
}

/// Tests that a lock is released after the file that owns it is dropped.
#[test]
fn lock_cleanup() {
    let (_dir, path) = tmpfile();
    let file1 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let file2 = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();

    file1.lock_exclusive().unwrap();
    assert_eq!(
        file2.try_lock_shared().unwrap_err().kind(),
        lock_contended_error().kind()
    );

    // Drop file1; the lock should be released.
    drop(file1);
    file2.lock_shared().unwrap();
}

/// Tests file allocation.
#[test]
fn allocate() {
    let (_dir, path) = tmpfile();
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(&path)
        .unwrap();
    let blksize = allocation_granularity(&path).unwrap();

    // New files are created with no allocated size.
    assert_eq!(0, file.allocated_size().unwrap());
    assert_eq!(0, file.metadata().unwrap().len());

    // Allocate space for the file, checking that the allocated size steps
    // up by block size, and the file length matches the allocated size.

    file.allocate(2 * blksize - 1).unwrap();
    assert_eq!(2 * blksize, file.allocated_size().unwrap());
    assert_eq!(2 * blksize - 1, file.metadata().unwrap().len());

    // Truncate the file, checking that the allocated size steps down by
    // block size.

    file.set_len(blksize + 1).unwrap();
    assert_eq!(2 * blksize, file.allocated_size().unwrap());
    assert_eq!(blksize + 1, file.metadata().unwrap().len());
}

/// Checks filesystem space methods.
#[test]
fn filesystem_space() {
    let tempdir = tmpdir();
    let total_space = total_space(&tempdir.path()).unwrap();
    let free_space = free_space(&tempdir.path()).unwrap();
    let available_space = available_space(&tempdir.path()).unwrap();

    assert!(total_space >= free_space);
    assert!(total_space >= available_space);
    assert!(available_space <= free_space);
}

// nightly-only benchmarks (due to libtest usage)
#[cfg(nightly)]
mod bench {

    extern crate test;

    use super::{tmpdir, tmpfile};
    use crate::*;
    use std::fs;

    #[bench]
    fn bench_file_create(b: &mut test::Bencher) {
        let (_dir, path) = tmpfile();

        b.iter(|| {
            fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .unwrap();
            fs::remove_file(&path).unwrap();
        });
    }

    /// Benchmarks creating a file, truncating it to 32MiB, and deleting it.
    #[bench]
    fn bench_file_truncate(b: &mut test::Bencher) {
        let (_dir, path) = tmpfile();
        let size = 32 * 1024 * 1024;

        b.iter(|| {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .unwrap();
            file.set_len(size).unwrap();
            fs::remove_file(&path).unwrap();
        });
    }

    /// Benchmarks creating a file, allocating 32MiB for it, and deleting it.
    #[bench]
    fn bench_file_allocate(b: &mut test::Bencher) {
        let (_dir, path) = tmpfile();
        let size = 32 * 1024 * 1024;

        b.iter(|| {
            let file = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .open(&path)
                .unwrap();
            file.allocate(size).unwrap();
            fs::remove_file(&path).unwrap();
        });
    }

    /// Benchmarks creating a file, allocating 32MiB for it, and deleting it.
    #[cfg(nightly)]
    #[bench]
    fn bench_allocated_size(b: &mut test::Bencher) {
        let (_dir, path) = tmpfile();
        let size = 32 * 1024 * 1024;

        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        file.allocate(size).unwrap();

        b.iter(|| {
            file.allocated_size().unwrap();
        });
    }

    /// Benchmarks duplicating a file descriptor or handle.
    #[bench]
    fn bench_duplicate(b: &mut test::Bencher) {
        let (_dir, path) = tmpfile();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();

        b.iter(|| test::black_box(file.duplicate().unwrap()));
    }

    /// Benchmarks locking and unlocking a file lock.
    #[bench]
    fn bench_lock_unlock(b: &mut test::Bencher) {
        let (_dir, path) = tmpfile();
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();

        b.iter(|| {
            file.lock_exclusive().unwrap();
            file.unlock().unwrap();
        });
    }

    /// Benchmarks the free space method.
    #[bench]
    fn bench_free_space(b: &mut test::Bencher) {
        let dir = tmpdir();
        b.iter(|| {
            test::black_box(free_space(&dir.path()).unwrap());
        });
    }

    /// Benchmarks the available space method.
    #[bench]
    fn bench_available_space(b: &mut test::Bencher) {
        let dir = tmpdir();
        b.iter(|| {
            test::black_box(available_space(&dir.path()).unwrap());
        });
    }

    /// Benchmarks the total space method.
    #[bench]
    fn bench_total_space(b: &mut test::Bencher) {
        let dir = tmpdir();
        b.iter(|| {
            test::black_box(total_space(&dir.path()).unwrap());
        });
    }
}
