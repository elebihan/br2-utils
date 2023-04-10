//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

//! Provide helpers for managing builds.

use std::{
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

use super::builder::{self, BuildStep, Builder};

/// Errors reported when managing builds.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Builder error: {0}")]
    Builder(#[from] builder::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Manages builds.
#[derive(Debug)]
pub struct Mason {
    storage: PathBuf,
}

impl Mason {
    /// Create a new mason, using `storage` as location for build definitions.
    pub fn new<P: AsRef<Path>>(storage: P) -> Self {
        Self {
            storage: storage.as_ref().to_path_buf(),
        }
    }

    /// Add a new build definition, created from a `Builder`
    pub fn add_from_builder(&self, name: &str, builder: &Builder) -> Result<(), Error> {
        if !self.storage.exists() {
            fs::create_dir_all(&self.storage)?;
        }
        let path = self.build_definition_path(name);
        let text = builder.to_toml()?;
        fs::write(path, text)?;
        Ok(())
    }

    /// List all available build definitions.
    pub fn list(&self) -> Result<Vec<String>, Error> {
        let dir = fs::read_dir(&self.storage)?;
        let names = dir
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter_map(|p| {
                if p.extension().map_or(false, |e| e == "toml") {
                    Some(p)
                } else {
                    None
                }
            })
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .collect::<Vec<String>>();
        Ok(names)
    }

    /// delete a build definition.
    pub fn delete(&self, name: &str) -> Result<(), Error> {
        let path = self.build_definition_path(name);
        fs::remove_file(path)?;
        Ok(())
    }

    /// Perform a build from a definition.
    pub fn build(&self, name: &str, step: BuildStep) -> Result<(), Error> {
        let builder = self.create_builder(name)?;
        builder.run_step(step)?;
        Ok(())
    }

    /// Build some specific targets of a build definition.
    pub fn execute<S: AsRef<str>>(&self, name: &str, targets: &[S]) -> Result<(), Error> {
        let builder = self.create_builder(name)?;
        builder.build_targets(targets)?;
        Ok(())
    }

    ///  Print contents of a build definition
    pub fn show(&self, name: &str) -> Result<(), Error> {
        let s = self.read_build_definition(name)?;
        println!("{s}");
        Ok(())
    }

    fn read_build_definition(&self, name: &str) -> Result<String, Error> {
        let path = self.build_definition_path(name);
        let s = fs::read_to_string(path)?;
        Ok(s)
    }

    fn create_builder(&self, name: &str) -> Result<Builder, Error> {
        let s = self.read_build_definition(name)?;
        let b = Builder::from_toml(&s)?;
        Ok(b)
    }

    fn build_definition_path(&self, name: &str) -> PathBuf {
        let mut path = self.storage.join(name);
        path.set_extension("toml");
        path
    }
}
