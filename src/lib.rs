//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

//! Provide helpers to handle a [Buildroot](https://buildroot.org) environment.

mod buildroot;
pub mod defconfig;
pub mod package;

pub use buildroot::*;
