//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

//! Provide helpers for building using a defconfig.

use std::{path::PathBuf, str::FromStr};

use thiserror::Error;

/// Errors reported when performing a build
#[derive(Debug, Error)]
pub enum Error {
    #[error("Build failed")]
    BuildFailed,
    #[error("Invalid step")]
    InvalidStep,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Represent a Buildroot builder
#[derive(Debug)]
pub struct Builder {
    pub(crate) defconfig: PathBuf,
    pub(crate) output: PathBuf,
    pub(crate) main: PathBuf,
    pub(crate) externals: Vec<PathBuf>,
}

/// Represent a build step
#[derive(Debug, Clone, Copy)]
pub enum BuildStep {
    /// Initialize and build
    All,
    /// Initialize a build using defconfig
    Init,
    /// Continue a previously initialized build
    Main,
}

impl FromStr for BuildStep {
    type Err = self::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "all" => Ok(BuildStep::All),
            "init" => Ok(BuildStep::Init),
            "main" => Ok(BuildStep::Main),
            _ => Err(Error::InvalidStep),
        }
    }
}

impl std::fmt::Display for BuildStep {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildStep::All => write!(f, "all"),
            BuildStep::Init => write!(f, "init"),
            BuildStep::Main => write!(f, "main"),
        }
    }
}

impl Builder {
    /// Run a build step
    pub fn run_step(&self, step: BuildStep) -> Result<(), Error> {
        let mut targets = vec![];
        match step {
            BuildStep::Init => targets.push("defconfig"),
            BuildStep::All => targets.extend_from_slice(&["defconfig", "all"]),
            BuildStep::Main => targets.push("all"),
        }
        // "defconfig" can not be batched with "all", so build each separately.
        for target in targets {
            self.build_targets(&[target])?;
        }
        Ok(())
    }

    /// Build a list of targets specified by name
    pub fn build_targets(&self, targets: &[&str]) -> Result<(), Error> {
        let mut cmd = std::process::Command::new("make");
        let external: String = self.externals.iter().fold(String::new(), |a, p| {
            a + ":" + &p.as_os_str().to_string_lossy()
        });
        if !external[1..].is_empty() {
            cmd.arg(format!("BR2_EXTERNAL={}", &external[1..]));
        }
        let defconfig = format!("BR2_DEFCONFIG={}", self.defconfig.to_string_lossy());
        let output = format!("O={}", self.output.to_string_lossy());
        cmd.arg("-C")
            .arg(self.main.as_os_str())
            .arg(output)
            .arg(defconfig);
        for target in targets {
            cmd.arg(target);
        }
        let status = cmd.status()?;
        status.success().then_some(()).ok_or(Error::BuildFailed)
    }
}
