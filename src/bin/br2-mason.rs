//
// This file is part of br2-utils
//
// SPDX-FileCopyrightText: © 2023 Eric Le Bihan <eric.le.bihan.dev@free.fr>
//
// SPDX-License-Identifier: MIT
//

use anyhow::{anyhow, Context, Result};
use br2_utils::mason::Mason;
use clap::{Parser, Subcommand};
use commands::{add::Add, build::Build, delete::Delete, execute::Execute, list::List, show::Show};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
enum Command {
    #[clap(visible_alias = "a")]
    Add(Add),
    #[clap(visible_alias = "b")]
    Build(Build),
    #[clap(visible_alias = "d")]
    Delete(Delete),
    #[clap(visible_alias = "e")]
    Execute(Execute),
    #[clap(visible_aliases = ["l", "ls"])]
    List(List),
    #[clap(visible_aliases = ["s", "sh"])]
    Show(Show),
}

#[derive(Debug, Parser)]
#[command(name = "br2-mason", version, about = "Manage Buildroot builds")]
struct Cli {
    #[arg(short, long, help = " Path to build definitions")]
    storage: Option<PathBuf>,
    #[command(subcommand, help = "Build command")]
    command: Command,
}

pub fn main() -> Result<()> {
    let args = Cli::parse();
    let storage = args
        .storage
        .or_else(utils::user_local_storage)
        .ok_or(anyhow!("No storage found"))?;
    let mason = Mason::new(storage);
    match args.command {
        Command::Add(ref cmd) => cmd
            .execute(&mason)
            .with_context(|| "Failed to add build definition")?,
        Command::Build(ref cmd) => cmd
            .execute(&mason)
            .with_context(|| "Failed to build using build definition")?,
        Command::Delete(ref cmd) => cmd
            .execute(&mason)
            .with_context(|| "Failed to delete build definition")?,
        Command::Execute(ref cmd) => cmd
            .execute(&mason)
            .with_context(|| "Failed to execute target(s)")?,
        Command::List(ref cmd) => cmd
            .execute(&mason)
            .with_context(|| "Failed to list build definitions")?,
        Command::Show(ref cmd) => cmd
            .execute(&mason)
            .with_context(|| "Failed to show build definition")?,
    }
    Ok(())
}

mod commands {
    pub mod add {
        use std::path::PathBuf;

        use anyhow::{Context, Error};
        use br2_utils::{mason::Mason, BuildrootExplorer};
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct Add {
            #[arg(short, long, help = "Path to main tree")]
            main: Option<PathBuf>,
            #[arg(
                short,
                long = "external",
                help = "Path to external tree",
                value_name = "EXTERNAL"
            )]
            externals: Vec<PathBuf>,
            #[arg(help = "Name of the build")]
            name: String,
            #[arg(help = "Name of the defconfig")]
            defconfig: String,
            #[arg(help = "Path to output directory")]
            output: PathBuf,
        }

        impl Add {
            pub fn execute(&self, mason: &Mason) -> Result<(), Error> {
                let cur_dir = std::env::current_dir()?;
                let main = self.main.as_ref().unwrap_or(&cur_dir);
                let mut explorer = BuildrootExplorer::new(main);
                for external in &self.externals {
                    explorer.external_tree(external);
                }
                let buildroot = explorer
                    .explore()
                    .with_context(|| "Failed to explore Buildroot tree")?;
                let builder = buildroot
                    .create_builder(&self.defconfig, &self.output)
                    .with_context(|| "Failed to create Buildroot builder")?;
                mason.add_from_builder(&self.name, &builder)?;
                Ok(())
            }
        }
    }
    pub mod build {
        use br2_utils::{
            builder::BuildStep,
            mason::{Error, Mason},
        };
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct Build {
            #[arg(short, long, help = "Build step", default_value_t = BuildStep::All)]
            step: BuildStep,
            #[arg(help = "Name of the build")]
            name: String,
        }

        impl Build {
            pub fn execute(&self, mason: &Mason) -> Result<(), Error> {
                mason.build(&self.name, self.step)
            }
        }
    }
    pub mod delete {
        use br2_utils::mason::{Error, Mason};
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct Delete {
            #[arg(help = "Name of the build")]
            name: String,
        }

        impl Delete {
            pub fn execute(&self, mason: &Mason) -> Result<(), Error> {
                mason.delete(&self.name)
            }
        }
    }
    pub mod execute {
        use br2_utils::mason::{Error, Mason};
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct Execute {
            #[arg(help = "Name of the build")]
            name: String,
            #[arg(help = "Name of the target to build", value_name = "TARGET")]
            targets: Vec<String>,
        }

        impl Execute {
            pub fn execute(&self, mason: &Mason) -> Result<(), Error> {
                mason.execute(&self.name, &self.targets)
            }
        }
    }
    pub mod list {
        use br2_utils::mason::{Error, Mason};
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct List;

        impl List {
            pub fn execute(&self, mason: &Mason) -> Result<(), Error> {
                for entry in mason.list()? {
                    println!("{entry}");
                }
                Ok(())
            }
        }
    }
    pub mod show {
        use br2_utils::mason::{Error, Mason};
        use clap::Args;

        #[derive(Debug, Args)]
        pub struct Show {
            #[arg(help = "Name of the build")]
            name: String,
        }

        impl Show {
            pub fn execute(&self, mason: &Mason) -> Result<(), Error> {
                mason.show(&self.name)
            }
        }
    }
}

mod utils {
    use std::path::PathBuf;

    pub fn user_local_storage() -> Option<PathBuf> {
        dirs::config_local_dir().map(|p| p.join("br2-utils"))
    }
}
