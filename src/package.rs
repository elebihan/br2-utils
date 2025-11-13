//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

//! Provide helpers for handling packages.

use regex::{Captures, Regex};
use std::borrow::Cow;
use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use thiserror::Error;

/// Errors reported when processing a package.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid filename: {0:?}")]
    InvalidFilename(OsString),
    #[error("Invalid variable: {0}")]
    InvalidVariable(String),
    #[error("Missing variable: {0}")]
    MissingVariable(String),
}

/// Hold information about a package.
#[derive(Debug)]
pub struct PackageInfo {
    name: String,
    properties: HashMap<&'static str, String>,
}

impl PackageInfo {
    /// Collect package information from file at `path`.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = File::open(&path)?;
        let name = path
            .as_ref()
            .file_stem()
            .map(|n| n.to_string_lossy())
            .ok_or(Error::InvalidFilename(path.as_ref().as_os_str().into()))?;
        Self::from_reader(&name, file)
    }

    /// Collect package information from a readable object.
    fn from_reader<R: Read>(name: &str, reader: R) -> Result<Self, Error> {
        let stem = canonicalize(name);
        let prop_names = ["version", "site", "source", "license", "dependencies"];
        let vars_names: Vec<(&str, String)> = prop_names
            .into_iter()
            .map(|n| (n, format!("{}_{}", stem, n.to_uppercase())))
            .collect();
        let mut properties = HashMap::new();
        let reader = BufReader::new(reader);
        for line in reader.lines() {
            let line = line?;
            for (prop_name, var_name) in &vars_names {
                if line.starts_with(var_name) {
                    let fields = line.split('=').collect::<Vec<&str>>();
                    if fields.len() != 2 {
                        return Err(Error::InvalidVariable(line));
                    }
                    properties.insert(*prop_name, fields[1].trim().to_string());
                }
            }
        }
        if !properties.contains_key("version") {
            return Err(Error::MissingVariable("version".to_string()));
        }
        Ok(Self {
            name: name.to_string(),
            properties,
        })
    }

    /// Return the name of the package.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Return the version of the package.
    pub fn version(&self) -> &str {
        &self.properties["version"]
    }

    /// Return the properties of a package.
    pub fn properties(&self) -> &HashMap<&'static str, String> {
        &self.properties
    }
}

/// Set the version of the package in `path` to `version`.
pub fn set_package_version<P: AsRef<Path>>(path: P, version: &str) -> Result<(), Error> {
    let name = path
        .as_ref()
        .file_stem()
        .map(|s| s.to_string_lossy())
        .ok_or_else(|| Error::InvalidFilename(path.as_ref().as_os_str().into()))?;
    let old_text = fs::read_to_string(&path)?;
    let new_text = replace_version(&old_text, &name, version);
    fs::write(&path, new_text.as_bytes())?;
    Ok(())
}

fn replace_version<'t>(text: &'t str, name: &str, version: &str) -> Cow<'t, str> {
    let pattern = format!(r"({}_VERSION\s*=\s*)(.+)", canonicalize(name));
    let regex = Regex::new(pattern.as_str()).unwrap();
    regex.replace(text, |caps: &Captures| format!("{}{}", &caps[1], version))
}

fn canonicalize(name: &str) -> String {
    name.to_uppercase().replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGE_VALID: &str = r##"
# Comment
FOO_VERSION    =   1.2.3
FOO_SITE   =   https://some.where/there
"##;
    const PACKAGE_NO_VERSION: &str = r##"
FOO_LICENSE = LGPL-2.0+
"##;

    #[test]
    fn parse_package_valid() {
        let res = PackageInfo::from_reader("foo", PACKAGE_VALID.as_bytes());
        assert!(res.is_ok());
        let pkg = res.unwrap();
        assert_eq!(pkg.version(), "1.2.3");
        assert_eq!(
            pkg.properties().get("site").map(String::as_str),
            Some("https://some.where/there")
        );
    }

    #[test]
    fn parse_package_invalid() {
        let res = PackageInfo::from_reader("foo", PACKAGE_NO_VERSION.as_bytes());
        assert!(res.is_err());
    }

    #[test]
    fn replace_version() {
        let old_text = PACKAGE_VALID.to_string();
        let new_text = super::replace_version(&old_text, "foo", "3.2.1");
        let info = PackageInfo::from_reader("foo", new_text.as_bytes()).unwrap();
        assert_eq!(info.version(), "3.2.1");
    }
}
