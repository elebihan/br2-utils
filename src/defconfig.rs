//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

//! Provide helpers for handling defconfig files.

use lazy_static::lazy_static;
use regex::Regex;
use thiserror::Error;

use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::Path,
    str::FromStr,
};

/// Errors reported when processing a defconfig file.
#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid kind: {0}")]
    InvalidValue(String),
    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),
}

/// Value of a symbol in a `Defconfig`.
#[derive(Debug, PartialEq)]
pub enum SymbolValue {
    Bool(bool),
    String(String),
}

impl FromStr for SymbolValue {
    type Err = self::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        if s.starts_with('\"') && s.ends_with('\"') {
            let s = s
                .trim_start_matches('\"')
                .trim_end_matches('\"')
                .to_string();
            return Ok(SymbolValue::String(s));
        }

        match s {
            "y" => Ok(SymbolValue::Bool(true)),
            "n" => Ok(SymbolValue::Bool(false)),
            _ => Err(Error::InvalidValue(s.to_string())),
        }
    }
}

/// Represent a symbol in a `Defconfig`.
#[derive(Debug, PartialEq)]
pub struct Symbol {
    pub name: String,
    pub value: SymbolValue,
}

impl FromStr for Symbol {
    type Err = self::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        lazy_static! {
            static ref SYMBOL_SET: Regex = Regex::new(r"(BR2_[a-zA-Z0-9_]+)=(.+)").unwrap();
            static ref SYMBOL_NOTSET: Regex =
                Regex::new(r"# (BR2_[a-zA-Z0-9_]+) is not set").unwrap();
        }

        if let Some(caps) = SYMBOL_SET.captures(s) {
            return Ok(Symbol {
                name: caps[1].to_string(),
                value: caps[2].parse::<SymbolValue>()?,
            });
        }

        if let Some(caps) = SYMBOL_NOTSET.captures(s) {
            return Ok(Symbol {
                name: caps[1].to_string(),
                value: SymbolValue::Bool(false),
            });
        }

        Err(Error::InvalidSymbol(s.to_string()))
    }
}

/// Hold information of a defconfig.
#[derive(Debug, PartialEq)]
pub struct Defconfig {
    symbols: Vec<Symbol>,
}

impl Defconfig {
    /// Construct a `Defconfig` from file at `path`.
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, Error> {
        let file = File::open(&path)?;
        Self::from_reader(file)
    }

    /// Construct a `Defconfig` from a readable object.
    pub fn from_reader<R: Read>(reader: R) -> Result<Self, Error> {
        let reader = BufReader::new(reader);
        let mut symbols = vec![];
        for line in reader.lines() {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            if line.starts_with('#') && !line.ends_with("is not set") {
                continue;
            }
            let symbol = line.parse::<Symbol>()?;
            symbols.push(symbol);
        }
        Ok(Self { symbols })
    }

    /// Return the list of symbols.
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// Check if a package is selected.
    pub fn selects(&self, package: &str) -> bool {
        let name = format!("BR2_PACKAGE_{}", package)
            .replace('-', "_")
            .to_uppercase();
        self.symbols.iter().any(|s| match s.value {
            SymbolValue::Bool(b) if b => s.name == name,
            _ => false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEFCONFIG_VALID: &str = r#"
# Comment
BR2_i386=y
BR2_PACKAGE_FOO=y
BR2_PACKAGE_FOO_BAR="1.2.3"
# BR2_PACKAGE_QUUX is not set
"#;

    fn reference_defconfig() -> Defconfig {
        Defconfig {
            symbols: vec![
                Symbol {
                    name: "BR2_i386".to_string(),
                    value: SymbolValue::Bool(true),
                },
                Symbol {
                    name: "BR2_PACKAGE_FOO".to_string(),
                    value: SymbolValue::Bool(true),
                },
                Symbol {
                    name: "BR2_PACKAGE_FOO_BAR".to_string(),
                    value: SymbolValue::String("1.2.3".to_string()),
                },
                Symbol {
                    name: "BR2_PACKAGE_QUUX".to_string(),
                    value: SymbolValue::Bool(false),
                },
            ],
        }
    }

    #[test]
    fn symbol_no_value() {
        let res = "BR2_PACKAGE_FOO".parse::<Symbol>();
        assert!(res.is_err());
    }

    #[test]
    fn symbol_invalid_not_selected() {
        let res = "#   BR2_PACKAGE_FOO    is   not set".parse::<Symbol>();
        assert!(res.is_err());
    }

    #[test]
    fn valid_defconfig() {
        let res = Defconfig::from_reader(DEFCONFIG_VALID.as_bytes());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), reference_defconfig());
    }

    #[test]
    fn does_select() {
        let defconfig = Defconfig::from_reader(DEFCONFIG_VALID.as_bytes()).unwrap();
        assert!(defconfig.selects("foo"));
    }

    #[test]
    fn does_not_select() {
        let defconfig = Defconfig::from_reader(DEFCONFIG_VALID.as_bytes()).unwrap();
        assert!(!defconfig.selects("bar"));
    }
}
