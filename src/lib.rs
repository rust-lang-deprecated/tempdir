// Copyright 2015 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![doc(html_logo_url = "https://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "https://www.rust-lang.org/favicon.ico",
       html_root_url = "https://doc.rust-lang.org/tempdir/")]
#![cfg_attr(test, deny(warnings))]

extern crate rand;

use std::env;
use std::io::{self, Error, ErrorKind};
use std::fmt;
use std::fs;
use std::path::{self, PathBuf, Path};
use rand::{thread_rng, Rng};

/// A wrapper for a path to temporary directory implementing automatic
/// scope-based deletion.
pub struct TempDir {
    path: Option<PathBuf>,
}

// How many times should we (re)try finding an unused random name? It should be
// enough that an attacker will run out of luck before we run out of patience.
const NUM_RETRIES: u32 = 1 << 31;
// How many characters should we include in a random file name? It needs to
// be enough to dissuade an attacker from trying to preemptively create names
// of that length, but not so huge that we unnecessarily drain the random number
// generator of entropy.
const NUM_RAND_CHARS: usize = 12;

impl TempDir {
    /// Creates a `TempDir` inside of `tmpdir` whose name
    /// will have the prefix `prefix`. The directory will be automatically
    /// deleted once the returned wrapper is destroyed.
    ///
    /// # Errors
    ///
    /// This function will return an error if no directory can be created.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use tempdir::TempDir;
    /// use std::path::Path;
    ///
    /// let file_path = Path::new("/tmp");
    /// let prefix = "my_prefix";
    ///
    /// let dir = TempDir::new_in(file_path, prefix).unwrap();
    /// let path = dir.path();
    ///
    /// use std::fs;
    /// assert!(fs::metadata(path).is_ok());
    /// ```
    pub fn new_in<P: AsRef<Path>>(tmpdir: P, prefix: &str) -> io::Result<TempDir> {
        let storage;
        let mut tmpdir = tmpdir.as_ref();
        if !tmpdir.is_absolute() {
            let cur_dir = try!(env::current_dir());
            storage = cur_dir.join(tmpdir);
            tmpdir = &storage;
            // return TempDir::new_in(&cur_dir.join(tmpdir), prefix);
        }

        let mut rng = thread_rng();
        for _ in 0..NUM_RETRIES {
            let suffix: String = rng.gen_ascii_chars().take(NUM_RAND_CHARS).collect();
            let leaf = if prefix.len() > 0 {
                format!("{}.{}", prefix, suffix)
            } else {
                // If we're given an empty string for a prefix, then creating a
                // directory starting with "." would lead to it being
                // semi-invisible on some systems.
                suffix
            };
            let path = tmpdir.join(&leaf);
            match fs::create_dir(&path) {
                Ok(_) => return Ok(TempDir { path: Some(path) }),
                Err(ref e) if e.kind() == ErrorKind::AlreadyExists => {}
                Err(e) => return Err(e),
            }
        }

        Err(Error::new(ErrorKind::AlreadyExists,
                       "too many temporary directories already exist"))
    }

    /// Creates a `TempDir` inside of `env::temp_dir()` whose
    /// name will have the prefix `prefix`. The directory will be automatically
    /// deleted once the returned wrapper is destroyed.
    ///
    /// # Errors
    ///
    /// This function will return an error if no directory can be created.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use tempdir::TempDir;
    /// use std::path::Path;
    ///
    /// let prefix = "my_prefix";
    ///
    /// let dir = TempDir::new(prefix).unwrap();
    /// let path: &Path = dir.path();
    ///
    /// use std::fs;
    /// assert!(fs::metadata(path).is_ok());
    /// ```
    pub fn new(prefix: &str) -> io::Result<TempDir> {
        TempDir::new_in(&env::temp_dir(), prefix)
    }

    /// Unwrap the wrapped `std::path::Path` from the `TempDir` wrapper.
    /// This discards the wrapper so that the automatic deletion of the
    /// temporary directory is prevented.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use tempdir::TempDir;
    /// use std::path::{self, PathBuf, Path};
    ///
    /// let prefix = "my_prefix";
    /// let dir = TempDir::new(prefix).unwrap();
    ///
    /// let path: PathBuf = dir.into_path();
    ///
    /// use std::fs;
    /// assert!(fs::metadata(path).is_ok());
    /// ```
    pub fn into_path(mut self) -> PathBuf {
        self.path.take().unwrap()
    }

    /// Access the wrapped `std::path::Path` to the temporary directory.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use tempdir::TempDir;
    /// use std::path::Path;
    ///
    /// let prefix = "my_prefix";
    /// let dir = TempDir::new(prefix).unwrap();
    ///
    /// let path: &Path = dir.path();
    ///
    /// use std::fs;
    /// assert!(fs::metadata(path).is_ok());
    /// ```
    pub fn path(&self) -> &path::Path {
        self.path.as_ref().unwrap()
    }

    /// Close and remove the temporary directory
    ///
    /// Although `TempDir` removes the directory on drop, in the destructor
    /// any errors are ignored. To detect errors cleaning up the temporary
    /// directory, call `close` instead.
    ///
    /// # Examples
    ///
    /// Basic usage:
    ///
    /// ```
    /// use tempdir::TempDir;
    ///
    /// let prefix = "my_prefix";
    /// let dir = TempDir::new(prefix).unwrap();
    /// let res = dir.close();
    ///
    /// assert!(res.is_ok());
    /// ```
    ///
    /// When the directory cannot be deleted for some reason
    ///
    /// ```
    /// use tempdir::TempDir;
    /// use std::fs;
    ///
    /// let prefix = "my_prefix";
    /// let dir = TempDir::new(prefix).unwrap();
    ///
    /// {
    ///     let path = dir.path();
    ///     fs::remove_dir(path);
    /// }
    ///
    /// let res = dir.close();
    ///
    /// assert!(res.is_err());
    ///
    /// ```
    pub fn close(mut self) -> io::Result<()> {
        self.cleanup_dir()
    }

    fn cleanup_dir(&mut self) -> io::Result<()> {
        match self.path {
            Some(ref p) => fs::remove_dir_all(p),
            None => Ok(()),
        }
    }
}

impl fmt::Debug for TempDir {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TempDir")
            .field("path", &self.path())
            .finish()
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = self.cleanup_dir();
    }
}

// the tests for this module need to change the path using change_dir,
// and this doesn't play nicely with other tests so these unit tests are located
// in src/test/run-pass/tempfile.rs
