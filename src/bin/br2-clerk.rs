//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

use anyhow::{Context, Result};
use br2_utils::BuildrootExplorer;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use topics::defconfig::Defconfig;
use topics::package::Package;

#[derive(Debug, Subcommand)]
enum Topic {
    #[clap(visible_aliases = ["d", "def"])]
    Defconfig(Defconfig),
    #[clap(visible_aliases = ["p", "pkg"])]
    Package(Package),
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
    #[command(subcommand, help = "Topic to handle")]
    topic: Topic,
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
    match args.topic {
        Topic::Defconfig(ref topic) => topic.execute(&buildroot)?,
        Topic::Package(ref topic) => topic.execute(&buildroot)?,
    }
    Ok(())
}

mod topics {
    pub mod defconfig {
        use br2_utils::{Buildroot, Error};
        use clap::{Args, Subcommand};
        use std::collections::BTreeSet;

        #[derive(Debug, Subcommand)]
        enum DefconfigCommand {
            /// List available defconfigs
            #[clap(visible_alias = "ls")]
            List,
        }

        #[derive(Debug, Args)]
        pub struct Defconfig {
            #[command(subcommand)]
            command: DefconfigCommand,
        }

        impl Defconfig {
            pub fn execute(&self, buildroot: &Buildroot) -> Result<(), Error> {
                match self.command {
                    DefconfigCommand::List => {
                        let items: BTreeSet<&String> =
                            buildroot.defconfigs().map(|(n, _)| n).collect();
                        for item in items {
                            println!("{item}");
                        }
                        Ok(())
                    }
                }
            }
        }
    }

    pub mod package {
        use br2_utils::{Buildroot, Error};
        use clap::{Args, Subcommand};
        use std::collections::{BTreeMap, BTreeSet};

        #[derive(Debug, Args)]
        struct ListArgs {
            #[arg(short, long, help = "Show details")]
            details: bool,
        }

        #[derive(Debug, Args)]
        struct BumpArgs {
            #[arg(required(true), help = "Name of the package to bump")]
            name: String,
            #[arg(required(true), help = "New version of the package")]
            version: String,
        }

        #[derive(Debug, Subcommand)]
        enum PackageCommand {
            /// List available packages
            #[clap(visible_alias = "ls")]
            List(ListArgs),
            /// Change version of a package
            #[clap(visible_alias = "b")]
            Bump(BumpArgs),
        }

        #[derive(Debug, Args)]
        pub struct Package {
            #[command(subcommand)]
            command: PackageCommand,
        }

        impl Package {
            pub fn execute(&self, buildroot: &Buildroot) -> Result<(), Error> {
                match self.command {
                    PackageCommand::List(ref args) => {
                        let pkg_names = buildroot.packages().map(|(n, _)| n);
                        if args.details {
                            let items: BTreeMap<&String, String> = pkg_names
                                .map(|n| {
                                    let v = buildroot
                                        .get_package_version(n)
                                        .unwrap_or("unknown".to_string());
                                    (n, v)
                                })
                                .collect();
                            for (n, v) in items {
                                println!("{n:<32} {v}");
                            }
                        } else {
                            let items: BTreeSet<&String> = pkg_names.collect();
                            for item in items {
                                println!("{item}");
                            }
                        }
                        Ok(())
                    }
                    PackageCommand::Bump(ref args) => {
                        buildroot.set_package_version(&args.name, &args.version)
                    }
                }
            }
        }
    }
}
