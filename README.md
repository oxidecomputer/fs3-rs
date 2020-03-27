# fs3

Extended utilities for working with files and filesystems in Rust.
`fs3` is a fork of [`fs2`](https://github.com/danburkert/fs2-rs).

[![Documentation](https://docs.rs/fs3/badge.svg)](https://docs.rs/fs3)
[![Crate](https://img.shields.io/crates/v/fs3.svg)](https://crates.io/crates/fs3)

## Features

- [x] file descriptor duplication.
- [x] file locks.
- [x] file (pre)allocation.
- [x] file allocation information.
- [x] filesystem space usage information.

## Platforms

`fs3` should work on any platform supported by
[`libc`](https://github.com/rust-lang/libc).

## Benchmarks

Simple benchmarks are provided for the methods provided. Many of these
benchmarks use files in a temporary directory. On many modern Linux distros the
default temporary directory, `/tmp`, is mounted on a tempfs filesystem, which
will have different performance characteristics than a disk-backed filesystem.
The temporary directory is configurable at runtime through the environment (see
[`env::temp_dir`](https://doc.rust-lang.org/stable/std/env/fn.temp_dir.html)).

## License

`fs3` is primarily distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE), [LICENSE-MIT](LICENSE-MIT) for details.

Copyright (c) 2015 Dan Burkert.

Copyright 2020 Oxide Computer Company
