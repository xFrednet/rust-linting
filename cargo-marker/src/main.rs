#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(clippy::index_refutable_slice)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::manual_let_else)] // Rustfmt doesn't like `let ... else {` rn

mod backend;
mod cli;
mod config;
mod utils;

use std::collections::HashMap;

use cli::{get_clap_config, Flags};
use config::Config;

const CARGO_ARGS_SEPARATOR: &str = "--";
const VERSION: &str = concat!("cargo-marker ", env!("CARGO_PKG_VERSION"));
const NO_LINTS_ERROR: &str = concat!(
    "Please provide at least one valid lint crate, ",
    "with the `--lints` argument, ",
    "or `[workspace.metadata.marker.lints]` in `Cargo.toml`"
);

#[derive(Debug)]
pub enum ExitStatus {
    /// The toolchain validation failed. This could happen, if rustup is not
    /// installed or the required toolchain is not installed.
    InvalidToolchain = 100,
    /// The execution of a tool, like rustup or cargo, failed.
    ToolExecutionFailed = 101,
    /// Unable to find the driver binary
    MissingDriver = 200,
    /// Nothing we can really do, but good to know. The user will have to analyze
    /// the forwarded cargo output.
    DriverInstallationFailed = 300,
    /// A general collection status, for failures originating from the driver
    DriverFailed = 400,
    /// The lint crate build failed for some reason
    LintCrateBuildFail = 500,
    /// Lint crate could not be found
    LintCrateNotFound = 501,
    /// The lint crate has been build, but the resulting binary could not be found.
    LintCrateLibNotFound = 502,
    /// Failed to fetch the lint crate
    LintCrateFetchFailed = 550,
    /// General "bad config" error
    BadConfiguration = 600,
    /// No lint crates were specified -> nothing to do
    NoLints = 601,
    /// Can't deserialise `workspace.metadata.marker.lints` properly
    WrongStructure = 602,
    /// An invalid configuration value was specified
    InvalidValue = 603,
    /// Check failed
    MarkerCheckFailed = 1000,
}

fn main() -> Result<(), ExitStatus> {
    let matches = get_clap_config().get_matches_from(
        std::env::args()
            .enumerate()
            .filter_map(|(index, value)| (!(index == 1 && value == "marker")).then_some(value))
            .take_while(|s| s != CARGO_ARGS_SEPARATOR),
    );

    let flags = Flags::from_args(&matches);

    if matches.get_flag("version") {
        print_version(&flags);
        return Ok(());
    }

    // TODO: Remove old implementation thingies
    // TODO: Probably next PR, but make uitest use the `cargo-marker` as a lib

    let config = match Config::try_from_manifest() {
        Ok(v) => Some(v),
        Err(e) => match e {
            config::ConfigFetchError::SectionNotFound => None,
            _ => return Err(e.emit_and_convert()),
        },
    };

    match matches.subcommand() {
        Some(("setup", args)) => {
            let rustc_flags = if flags.forward_rust_flags {
                std::env::var("RUSTFLAGS").unwrap_or_default()
            } else {
                String::new()
            };
            backend::driver::install_driver(args.get_flag("auto-install-toolchain"), flags.dev_build, &rustc_flags)
        },
        Some(("check", args)) => run_check(args, config, &flags),
        None => run_check(&matches, config, &flags),
        _ => unreachable!(),
    }
}

fn run_check(args: &clap::ArgMatches, config: Option<Config>, flags: &Flags) -> Result<(), ExitStatus> {
    // determine lints
    let mut lints = HashMap::new();
    let deps = if let Some(deps) = cli::collect_lint_deps(args) {
        deps
    } else if let Some(config) = config {
        config.lints
    } else {
        HashMap::new()
    };
    for (name, dep) in deps {
        lints.insert(name, dep.to_dep_entry());
    }

    // Validation
    if lints.is_empty() {
        eprintln!("{NO_LINTS_ERROR}");
        return Err(ExitStatus::NoLints);
    }

    // Configure backend
    let toolchain = backend::toolchain::Toolchain::try_find_toolchain(flags.dev_build, flags.verbose)?;
    let backend_conf = backend::Config {
        dev_build: flags.dev_build,
        lints,
        ..backend::Config::try_base_from(toolchain)?
    };

    // Run backend
    let additional_cargo_args: Vec<_> = std::env::args()
        .skip_while(|c| c != CARGO_ARGS_SEPARATOR)
        .skip(1)
        .collect();
    backend::run_check(&backend_conf, &additional_cargo_args)
}

fn print_version(flags: &Flags) {
    println!("cargo-marker version: {}", env!("CARGO_PKG_VERSION"));

    if flags.verbose {
        backend::driver::print_driver_version(flags.dev_build);
    }
}
