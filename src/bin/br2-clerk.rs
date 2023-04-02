//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

use std::path::PathBuf;

use anyhow::{Context, Result};
use br2_utils::BuildrootExplorer;
use clap::{Args, Parser, Subcommand};

#[derive(Debug, Args)]
struct ListArgs {
    object: commands::Object,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(visible_alias = "ls")]
    List(ListArgs),
}

#[derive(Debug, Parser)]
#[command(
    name = "br2-clerk",
    version,
    about = "Provide information or perform tasks on Buildroot environment"
)]
struct Cli {
    #[arg(short, long, help = "Path to main tree")]
    main: Option<PathBuf>,
    #[arg(short, long, help = "Path to external tree")]
    externals: Vec<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
pub fn main() -> Result<()> {
    let args = Cli::parse();
    let cur_dir = std::env::current_dir().with_context(|| "Failed to get current directory")?;
    let path = args.main.unwrap_or(cur_dir);
    let mut explorer = BuildrootExplorer::new(path);
    for path in &args.externals {
        explorer.external_tree(path);
    }
    let buildroot = explorer
        .explore()
        .with_context(|| "Failed to explore environment")?;
    match args.command {
        Command::List(args) => commands::list(&buildroot, args.object)?,
    }
    Ok(())
}

mod commands {
    use br2_utils::{Buildroot, Error};
    use clap::ValueEnum;
    use std::collections::BTreeSet;

    #[derive(Debug, Clone, ValueEnum)]
    pub enum Object {
        Defconfigs,
        Packages,
    }

    impl std::fmt::Display for Object {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            self.to_possible_value()
                .expect("Skipped value")
                .get_name()
                .fmt(f)
        }
    }

    pub fn list(buildroot: &Buildroot, object: Object) -> Result<(), Error> {
        let items: BTreeSet<&String> = match object {
            Object::Defconfigs => buildroot.defconfigs().map(|(n, _)| n).collect(),
            Object::Packages => buildroot.packages().map(|(n, _)| n).collect(),
        };
        for item in items {
            println!("{item}");
        }
        Ok(())
    }
}
