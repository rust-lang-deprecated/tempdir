tempdir
=======

A Rust library for creating a temporary directory and deleting its entire
contents when the directory is dropped.

[![Build Status](https://travis-ci.org/rust-lang/tempdir.svg?branch=master)](https://travis-ci.org/rust-lang/tempdir)

[Documentation](http://doc.rust-lang.org/tempdir)

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
tempdir = "0.3"
```

and this to your crate root:

```rust
extern crate tempdir;
```
