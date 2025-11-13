//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;
use walkdir::{DirEntry, WalkDir};

use super::{
    builder::{self, BuildStep, Builder},
    defconfig::{self, Defconfig},
    package,
};

const BUILDROOT_SUBDIRS: [&str; 8] = [
    "board",
    "boot",
    "configs",
    "fs",
    "linux",
    "package",
    "toolchain",
    "utils",
];

/// Errors reported when processing a Buildroot environment.
#[derive(Debug, Error)]
pub enum Error {
    #[error("Build error: {0}")]
    Build(#[from] builder::Error),
    #[error("Defconfig error: {0}")]
    Defconfig(#[from] defconfig::Error),
    #[error("Directory traversal error: {0}")]
    DirectoryTraversal(#[from] walkdir::Error),
    #[error("Invalid external tree manifest: {0:?}")]
    InvalidExternalTreeManifest(PathBuf),
    #[error("Invalid Buildroot tree: {0:?}")]
    InvalidBuildrootTree(PathBuf),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Package error: {0}")]
    Package(#[from] package::Error),
    #[error("Unknown defconfig: {0}")]
    UnknownDefconfig(String),
    #[error("Unknown package: {0}")]
    UnknownPackage(String),
}

/// Information about a Buidlroot external tree.
#[derive(Debug, Default)]
struct ExternalTreeInfo {
    name: String,
    desc: String,
}

impl ExternalTreeInfo {
    /// Build information about Buildroot external tree from `external.desc` file.
    fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let contents = fs::read_to_string(&path)?;
        let mut info = ExternalTreeInfo::default();
        for line in contents.lines() {
            let fields = line.split(':').collect::<Vec<&str>>();
            if fields.len() != 2 {
                return Err(Error::InvalidExternalTreeManifest(
                    path.as_ref().to_path_buf(),
                ));
            }
            let value = fields[1].trim().to_string();
            match fields[0] {
                "name" => {
                    info.name = value;
                }
                "desc" => {
                    info.desc = value;
                }
                _ => {
                    return Err(Error::InvalidExternalTreeManifest(
                        path.as_ref().to_path_buf(),
                    ))
                }
            }
        }
        Ok(info)
    }
}

#[derive(Debug)]
struct BuildrootBaseTree {
    #[allow(dead_code)]
    path: PathBuf,
    defconfigs: HashMap<String, PathBuf>,
    packages: HashMap<String, PathBuf>,
}

fn is_defconfig(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|f| f.ends_with("_defconfig"))
        .unwrap_or(false)
}

fn is_package(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|f| f.ends_with(".mk"))
        .unwrap_or(false)
}

impl BuildrootBaseTree {
    fn from_path<P: AsRef<Path>>(path: P) -> Result<BuildrootBaseTree, Error> {
        let path = path.as_ref();
        let cfg_dir = path.join("configs");
        let defconfigs = if cfg_dir.exists() {
            BuildrootBaseTree::collect_defconfigs(&cfg_dir)?
        } else {
            HashMap::new()
        };
        let packages = BuildrootBaseTree::collect_packages(path.join("package"))?;
        Ok(Self {
            path: path.to_path_buf(),
            defconfigs,
            packages,
        })
    }

    fn collect_defconfigs<P: AsRef<Path>>(path: P) -> Result<HashMap<String, PathBuf>, Error> {
        let mut defconfigs = HashMap::new();
        for entry in WalkDir::new(path).into_iter() {
            let entry = entry?;
            if is_defconfig(&entry) {
                defconfigs.insert(
                    entry.file_name().to_string_lossy().to_string(),
                    entry.into_path(),
                );
            }
        }
        Ok(defconfigs)
    }

    fn collect_packages<P: AsRef<Path>>(path: P) -> Result<HashMap<String, PathBuf>, Error> {
        let mut packages = HashMap::new();
        for entry in WalkDir::new(path).into_iter() {
            let entry = entry?;
            if is_package(&entry) {
                let path = entry.into_path();
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                packages.insert(name, path);
            }
        }
        Ok(packages)
    }
}

#[derive(Debug)]
enum BuildrootTree {
    Main(BuildrootBaseTree),
    #[allow(unused)]
    External(String, BuildrootBaseTree),
}

impl BuildrootTree {
    fn from_path(path: &BuildrootTreePath) -> Result<BuildrootTree, Error> {
        match path {
            BuildrootTreePath::Main(p) => BuildrootTree::main_from_path(p),
            BuildrootTreePath::External(p) => BuildrootTree::external_from_path(p),
        }
    }

    fn external_from_path<P: AsRef<Path>>(path: P) -> Result<BuildrootTree, Error> {
        let ext_info_path = path.as_ref().join("external.desc");
        let ext_info = ExternalTreeInfo::from_path(ext_info_path)?;
        let tree = BuildrootBaseTree::from_path(&path)?;
        Ok(BuildrootTree::External(ext_info.name, tree))
    }

    fn main_from_path<P: AsRef<Path>>(path: P) -> Result<BuildrootTree, Error> {
        if BUILDROOT_SUBDIRS
            .iter()
            .any(|d| !path.as_ref().join(d).is_dir())
        {
            return Err(Error::InvalidBuildrootTree(path.as_ref().to_path_buf()));
        }
        let tree = BuildrootBaseTree::from_path(&path)?;
        Ok(BuildrootTree::Main(tree))
    }
}

/// Represent a Buildroot environment, with all defconfigs and packages.
#[derive(Debug)]
pub struct Buildroot {
    trees: Vec<BuildrootTree>,
}

impl Buildroot {
    /// Return an iterator over the name and the path of defconfig files.
    pub fn defconfigs(&self) -> impl Iterator<Item = (&String, &PathBuf)> {
        self.trees.iter().flat_map(|t| match t {
            BuildrootTree::Main(t) => t.defconfigs.iter(),
            BuildrootTree::External(_, t) => t.defconfigs.iter(),
        })
    }

    /// Return an iterator over the name and the path of package files.
    pub fn packages(&self) -> impl Iterator<Item = (&String, &PathBuf)> {
        self.trees.iter().flat_map(|t| match t {
            BuildrootTree::Main(t) => t.packages.iter(),
            BuildrootTree::External(_, t) => t.packages.iter(),
        })
    }

    /// Return the version of a package named `name`
    pub fn get_package_version(&self, name: &str) -> Result<String, Error> {
        let path = self
            .packages()
            .find(|(n, _)| n.as_str() == name)
            .map(|(_, p)| p)
            .ok_or_else(|| Error::UnknownPackage(name.to_string()))?;
        let pkg = package::PackageInfo::from_path(path)?;
        let version = pkg.version();
        Ok(version.to_string())
    }

    /// Set the version of the package named `name` to `version`
    pub fn set_package_version(&self, name: &str, version: &str) -> Result<(), Error> {
        let path = self
            .packages()
            .find(|(n, _)| n.as_str() == name)
            .map(|(_, p)| p)
            .ok_or_else(|| Error::UnknownPackage(name.to_string()))?;
        package::set_package_version(path, version)?;
        Ok(())
    }

    /// Return information from a defconfig named `name`.
    pub fn get_defconfig(&self, name: &str) -> Result<Defconfig, Error> {
        self.defconfigs()
            .find(|(n, _)| n.as_str() == name)
            .ok_or(Error::UnknownDefconfig(name.to_string()))
            .and_then(|(_, p)| Ok(defconfig::Defconfig::from_path(p)?))
    }

    /// Create a builder for a given defconfig
    pub fn create_builder<P: AsRef<Path>>(&self, name: &str, output: P) -> Result<Builder, Error> {
        let defconfig = self
            .defconfigs()
            .find(|(n, _)| n.as_str() == name)
            .ok_or(Error::UnknownDefconfig(name.to_string()))
            .map(|(_, p)| p.into())?;
        let main = self.main_tree_path().to_path_buf();
        let externals = self
            .trees
            .iter()
            .skip(1)
            .filter_map(|t| {
                if let BuildrootTree::External(_, t) = t {
                    Some(t.path.to_path_buf())
                } else {
                    None
                }
            })
            .collect();
        Ok(Builder {
            defconfig,
            output: output.as_ref().to_path_buf(),
            main,
            externals,
        })
    }

    /// Build an embedded firmware
    pub fn build<P: AsRef<Path>>(
        &self,
        name: &str,
        output: P,
        step: BuildStep,
    ) -> Result<(), Error> {
        let builder = self.create_builder(name, output)?;
        builder.run_step(step)?;
        Ok(())
    }

    /// Return the path to the main tree
    fn main_tree_path(&self) -> &Path {
        if let BuildrootTree::Main(m) = &self.trees[0] {
            m.path.as_path()
        } else {
            unreachable!()
        }
    }
}

#[derive(Debug)]
enum BuildrootTreePath {
    Main(PathBuf),
    External(PathBuf),
}

/// A `Buildroot` builder, taking external source trees into account.
#[derive(Debug)]
pub struct BuildrootExplorer {
    paths: Vec<BuildrootTreePath>,
}

impl BuildrootExplorer {
    /// Construct a new `BuildrootExplorer` using `path` as the main Buildroot directory.
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = BuildrootTreePath::Main(path.as_ref().to_path_buf());
        let paths = vec![path];
        Self { paths }
    }

    /// Add `path` as an external source tree to be explored.
    pub fn external_tree<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        let path = BuildrootTreePath::External(path.as_ref().to_path_buf());
        self.paths.push(path);
        self
    }

    /// Explore all the source trees and consume the `BuildrootExplorer`, providing a `Buildroot` in return.
    pub fn explore(self) -> Result<Buildroot, Error> {
        let trees: Result<Vec<BuildrootTree>, Error> =
            self.paths.iter().map(BuildrootTree::from_path).collect();
        Ok(Buildroot { trees: trees? })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use tempfile::Builder;

    const TEMPLATE_PACKAGE: &str = r##"
@NAME@_VERSION = 1.2.3

@NAME@_SITE = http://some/where

"##;
    const TEMPLATE_CONFIG: &str = r##"
BR2_PACKAGE_FOO=y
# BR2_PACKAGE_BAR is not set
"##;

    const BUILDROOT_TEST_DIR: &str = "br2-utils-test";

    fn mock_config<P: AsRef<Path>>(dir: P, name: &str) -> std::io::Result<()> {
        let path = dir.as_ref().join(format!("{}_defconfig", name));
        fs::write(path, TEMPLATE_CONFIG)
    }

    fn mock_configs<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
        for name in ["acme_quux", "frob_wuz"] {
            mock_config(&dir, name)?;
        }
        Ok(())
    }

    fn mock_package<P: AsRef<Path>>(dir: P, name: &str) -> std::io::Result<()> {
        let contents = TEMPLATE_PACKAGE.replace("@NAME@", &name.to_uppercase());
        let mut path = dir.as_ref().join(name);
        fs::create_dir(&path)?;
        path.push(name);
        path.set_extension("mk");
        fs::write(path, contents)
    }

    fn mock_packages<P: AsRef<Path>>(dir: P) -> std::io::Result<()> {
        for name in ["foo", "bar"] {
            mock_package(&dir, name)?;
        }
        Ok(())
    }

    fn mock_tree<P: AsRef<Path>>(path: P) -> std::io::Result<()> {
        for dir in BUILDROOT_SUBDIRS {
            let path = path.as_ref().join(dir);
            fs::create_dir(&path)?;
            match dir {
                "configs" => mock_configs(&path)?,
                "package" => mock_packages(&path)?,
                _ => {}
            }
        }
        Ok(())
    }

    #[test]
    fn check_valid_buildroot() {
        let path = Builder::new().prefix(BUILDROOT_TEST_DIR).tempdir().unwrap();
        mock_tree(&path).unwrap();
        let res = BuildrootExplorer::new(&path).explore();
        let buildroot = res.unwrap();
        let mut defconfigs: Vec<&str> = buildroot.defconfigs().map(|(n, _)| n.as_str()).collect();
        defconfigs.sort();
        assert_eq!(defconfigs, ["acme_quux_defconfig", "frob_wuz_defconfig"]);
        let mut packages: Vec<&str> = buildroot.packages().map(|(n, _)| n.as_str()).collect();
        packages.sort();
        assert_eq!(packages, ["bar", "foo"]);
    }

    #[test]
    fn get_package_version() {
        let path = Builder::new().prefix(BUILDROOT_TEST_DIR).tempdir().unwrap();
        mock_tree(&path).unwrap();
        let buildroot = BuildrootExplorer::new(&path).explore().unwrap();
        assert_eq!(buildroot.get_package_version("foo").unwrap(), "1.2.3");
    }

    #[test]
    fn bump_package_version() {
        let path = Builder::new().prefix(BUILDROOT_TEST_DIR).tempdir().unwrap();
        mock_tree(&path).unwrap();
        let buildroot = BuildrootExplorer::new(&path).explore().unwrap();
        let res = buildroot.set_package_version("foo", "3.2.1");
        assert!(res.is_ok());
    }

    #[test]
    fn check_package_not_selected() {
        let path = Builder::new().prefix(BUILDROOT_TEST_DIR).tempdir().unwrap();
        mock_tree(&path).unwrap();
        let buildroot = BuildrootExplorer::new(&path).explore().unwrap();
        let res = buildroot.get_defconfig("acme_quux_defconfig");
        assert!(res.is_ok());
        let defconfig = res.unwrap();
        assert!(!defconfig.selects("bar"));
    }
}
