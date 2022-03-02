#![deny(unused_must_use)]

#[macro_use]
extern crate mouse;

use std::ffi::OsString;
use std::fs::{DirEntry, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;

use clap::*;
use merovingian::non_minable_models::ExitCode;
use mouse::error::{Context, Result};
use mouse::log::*;
use serde::{Deserialize, Serialize};

#[derive(Clap, Debug)]
#[clap(version, about, author)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Command,
    #[clap(long, short, parse(from_os_str), default_value = "configs/logs.yaml")]
    /// Path to log config file.
    pub log_config_path: PathBuf,
}

#[derive(Clap, Debug)]
pub enum Command {
    Bump {
        #[clap(short)]
        /// Package name
        package: String,
        #[clap(short, arg_enum)]
        /// Semantic version level: Major, Minor, Patch
        kind: BumpKind,
    },
    /// Starts program and reloads it when necessary.
    Start(StartArgs),
}

#[derive(Clap, Debug)]
pub struct StartArgs {
    #[clap(short)]
    /// Package name
    pub package: String,
    #[clap()]
    /// Command line arguments for starting a program.
    pub args: Vec<OsString>,
}

#[derive(Clap, Debug, PartialEq)]
pub enum BumpKind {
    Major,
    Minor,
    Patch,
}

#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    release_path: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    unsafe {
        config::load(&args.log_config_path)?;
    }
    match args.command {
        Command::Bump { package, kind } => {
            bump(package, kind)?;
        }
        Command::Start(start_args) => {
            start(start_args)?;
        }
    }
    Ok(())
}

fn start(start_args: StartArgs) -> Result<()> {
    let settings: Settings = serde_yaml::from_reader(File::open("the_architect_settings.yaml")?)?;
    loop {
        let path = Path::new(&settings.release_path).join(&start_args.package);
        let mut child = None;
        for entry in path.read_dir()? {
            let entry = entry?;
            if entry
                .file_name()
                .to_str()
                .unwrap()
                .contains(path.file_name().unwrap().to_str().unwrap())
            {
                let child_path = entry.path().to_str().unwrap().to_string();
                info!("Starting '{}'...", child_path);
                let mut command = std::process::Command::new(&child_path);
                command.args(&start_args.args);
                child = Some(command.spawn()?);
            }
        }
        match child {
            None => {
                error!("program '{}' not found", start_args.package);
                throw!("program '{}' not found", start_args.package);
            }
            Some(mut c) => {
                let status = c.wait()?;
                match status.code() {
                    None => {}
                    Some(exit_code) => {
                        if exit_code == ExitCode::Success as i32 {
                            info!("Program '{}' successfully ended.", start_args.package);
                        } else if exit_code == ExitCode::FailedSafely as i32 {
                            warn!("Program '{}' ended with error that does not require immediate action.", start_args.package);
                        } else if exit_code == ExitCode::Reload as i32 {
                            info!("Program '{}' is reloading.", start_args.package);
                            continue;
                        } else if exit_code == ExitCode::Fatal as i32 {
                            error!("Program '{}' FAILED with FATAL exit code. Immediate action is required!", start_args.package);
                        } else {
                            error!("Program '{}' FAILED with unknown exit code. Immediate action is required!", start_args.package);
                        }
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn bump(package: String, kind: BumpKind) -> Result<()> {
    let dir = std::fs::read_dir(Path::new("."))?;
    for entry in dir {
        let entry = entry?;
        if entry.file_name().to_str().unwrap() == package && entry.file_type()?.is_dir() {
            let version = bump_cargo_toml(&entry, kind)?;
            // Must be here
            println!("{}", version);
            break;
        }
    }
    Ok(())
}

fn bump_cargo_toml(entry: &DirEntry, kind: BumpKind) -> Result<String> {
    let cargo_path = entry.path().join("Cargo.toml");
    let mut cargo_file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&cargo_path)?;
    let mut cargo = String::new();
    cargo_file.read_to_string(&mut cargo)?;

    const VERSION_STR: &str = "version = \"";
    let package_i =
        find_pattern(&cargo, "[package]").context(cargo_path.to_str().unwrap().to_string())?;
    let version_i = find_pattern(&cargo[package_i..], VERSION_STR)
        .context(cargo_path.to_str().unwrap().to_string())?;
    let start = package_i + version_i + VERSION_STR.len();
    let end = cargo[start..].find("\"").unwrap() + start;
    let mut version = semver::Version::from_str(&cargo[start..end])?;
    match kind {
        BumpKind::Major => version.increment_major(),
        BumpKind::Minor => version.increment_minor(),
        BumpKind::Patch => version.increment_patch(),
    }

    let version = version.to_string();
    cargo.replace_range(start..end, &version);
    cargo_file.set_len(0)?;
    cargo_file.seek(SeekFrom::Start(0))?;
    cargo_file.write_all(cargo.as_bytes())?;
    Ok(version)
}

fn find_pattern(contents: &str, pattern: &str) -> Result<usize> {
    let version_i = match contents.find(pattern) {
        None => throw!("file, doesn't have {}", pattern),
        Some(i) => i,
    };
    Ok(version_i)
}
