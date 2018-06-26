//! Build script for Rust modules in Tor.
//!
//! We need to use this because some of our Rust tests need to use some
//! of our C modules, which need to link some external libraries.
//!
//! This script works by looking at a "config.rust" file generated by our
//! configure script, and then building a set of options for cargo to pass to
//! the compiler.

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io;
use std::path::PathBuf;

/// Wrapper around a key-value map.
struct Config(
    HashMap<String,String>
);

/// Locate a config.rust file generated by autoconf, starting in the OUT_DIR
/// location provided by cargo and recursing up the directory tree.  Note that
/// we need to look in the OUT_DIR, since autoconf will place generated files
/// in the build directory.
fn find_cfg() -> io::Result<String> {
    let mut path = PathBuf::from(env::var("OUT_DIR").unwrap());
    loop {
        path.push("config.rust");
        if path.exists() {
            return Ok(path.to_str().unwrap().to_owned());
        }
        path.pop(); // remove config.rust
        if ! path.pop() { // can't remove last part of directory
            return Err(io::Error::new(io::ErrorKind::NotFound,
                                      "No config.rust"));
        }
    }
}

impl Config {
    /// Find the config.rust file and try to parse it.
    ///
    /// The file format is a series of lines of the form KEY=VAL, with
    /// any blank lines and lines starting with # ignored.
    fn load() -> io::Result<Config> {
        let path = find_cfg()?;
        let f = File::open(&path)?;
        let reader = io::BufReader::new(f);
        let mut map = HashMap::new();
        for line in reader.lines() {
            let s = line?;
            if s.trim().starts_with("#") || s.trim() == "" {
                continue;
            }
            let idx = match s.find("=") {
                None => {
                    return Err(io::Error::new(io::ErrorKind::InvalidData,
                                              "missing ="));
                },
                Some(x) => x
            };
            let (var,eq_val) = s.split_at(idx);
            let val = &eq_val[1..];
            map.insert(var.to_owned(), val.to_owned());
        }
        Ok(Config(map))
    }

    /// Return a reference to the value whose key is 'key'.
    ///
    /// Panics if 'key' is not found in the configuration.
    fn get(&self, key : &str) -> &str {
        self.0.get(key).unwrap()
    }

    /// Add a dependency on a static C library that is part of Tor, by name.
    fn component(&self, s : &str) {
        println!("cargo:rustc-link-lib=static={}", s);
    }

    /// Add a dependency on a native library that is not part of Tor, by name.
    fn dependency(&self, s : &str) {
        println!("cargo:rustc-link-lib={}", s);
    }

    /// Add a link path, relative to Tor's build directory.
    fn link_relpath(&self, s : &str) {
        let builddir = self.get("BUILDDIR");
        println!("cargo:rustc-link-search=native={}/{}", builddir, s);
    }

    /// Add an absolute link path.
    fn link_path(&self, s : &str) {
        println!("cargo:rustc-link-search=native={}", s);
    }

    /// Parse the CFLAGS in s, looking for -l and -L items, and adding
    /// rust configuration as appropriate.
    fn from_cflags(&self, s : &str) {
        let mut next_is_lib = false;
        let mut next_is_path = false;
        for ent in self.get(s).split_whitespace() {
            if next_is_lib {
                self.dependency(ent);
                next_is_lib = false;
            } else if next_is_path {
                self.link_path(ent);
                next_is_path = false;
            } else if ent == "-l" {
                next_is_lib = true;
            } else if ent == "-L" {
                next_is_path = true;
            } else if ent.starts_with("-L") {
                self.link_path(&ent[2..]);
            } else if ent.starts_with("-l") {
                self.dependency(&ent[2..]);
            }
        }
    }
}

pub fn main() {
    let cfg = Config::load().unwrap();
    let package = env::var("CARGO_PKG_NAME").unwrap();

    match package.as_ref() {
        "crypto" => {
            // Right now, I'm having a separate configuration for each Rust
            // package, since I'm hoping we can trim them down.  Once we have a
            // second Rust package that needs to use this build script, let's
            // extract some of this stuff into a module.
            //
            // This is a ridiculous amount of code to be pulling in just
            // to test our crypto library: modularity would be our
            // friend here.
            cfg.from_cflags("TOR_LDFLAGS_zlib");
            cfg.from_cflags("TOR_LDFLAGS_openssl");
            cfg.from_cflags("TOR_LDFLAGS_libevent");

            cfg.link_relpath("src/lib");
            cfg.link_relpath("src/common");
            cfg.link_relpath("src/ext/keccak-tiny");
            cfg.link_relpath("src/ext/ed25519/ref10");
            cfg.link_relpath("src/ext/ed25519/donna");
            cfg.link_relpath("src/trunnel");

            // Note that we can't pull in "libtor-testing", or else we
            // will have dependencies on all the other rust packages that
            // tor uses.  We must be careful with factoring and dependencies
            // moving forward!
            cfg.component("tor-crypt-ops-testing");
            cfg.component("or-testing");
            cfg.component("tor-log");
            cfg.component("tor-lock");
            cfg.component("tor-fdio");
            cfg.component("tor-container-testing");
            cfg.component("tor-smartlist-core-testing");
            cfg.component("tor-string-testing");
            cfg.component("tor-malloc");
            cfg.component("tor-wallclock");
            cfg.component("tor-err-testing");
            cfg.component("or-event-testing");
            cfg.component("tor-intmath-testing");
            cfg.component("tor-ctime-testing");
            cfg.component("curve25519_donna");
            cfg.component("keccak-tiny");
            cfg.component("ed25519_ref10");
            cfg.component("ed25519_donna");
            cfg.component("or-trunnel-testing");

            cfg.from_cflags("TOR_ZLIB_LIBS");
            cfg.from_cflags("TOR_LIB_MATH");
            cfg.from_cflags("TOR_OPENSSL_LIBS");
            cfg.from_cflags("TOR_LIBEVENT_LIBS");
            cfg.from_cflags("TOR_LIB_WS32");
            cfg.from_cflags("TOR_LIB_GDI");
            cfg.from_cflags("TOR_LIB_USERENV");
            cfg.from_cflags("CURVE25519_LIBS");
            cfg.from_cflags("TOR_LZMA_LIBS");
            cfg.from_cflags("TOR_ZSTD_LIBS");
            cfg.from_cflags("LIBS");
        },
        _ => {
            panic!("No configuration in build.rs for package {}", package);
        }
    }
}
