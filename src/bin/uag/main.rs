#![doc = include_str!("../../../README.md")]

mod broken;
mod build;
mod info;
mod outside;
mod unused;

use crate::{broken::BrokenArgs, build::BuildArgs, info::InfoArgs, outside::OutsideArgs, unused::UnusedArgs};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Builds a database of the relationships between all the assets and scripts in a Unity project,
/// and supports a set of useful queries against that database.
#[derive(Parser)]
#[command(version, about, long_about)]
pub struct CliArgs {
    #[command(subcommand)]
    command: CliCommand,

    /// Path to the database file
    #[arg(long, short, default_value = "db.bin")]
    pub db_path: PathBuf,
}

#[derive(Subcommand)]
enum CliCommand {
    Build(BuildArgs),
    Info(InfoArgs),
    Unused(UnusedArgs),
    Broken(BrokenArgs),
    Outside(OutsideArgs),
}

impl CliArgs {
    pub fn run(&self) {
        match &self.command {
            CliCommand::Build(args) => args.run(self),
            CliCommand::Info(args) => args.run(self),
            CliCommand::Unused(args) => args.run(self),
            CliCommand::Broken(args) => args.run(self),
            CliCommand::Outside(args) => args.run(self),
        };
    }
}

fn main() {
    CliArgs::parse().run();
}
