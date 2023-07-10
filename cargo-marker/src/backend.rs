//! The backend is the brains of rust-marker, it's responsible for installing or
//! finding the correct driver, building lints and start linting. The backend should
//! be decoupled from the frontend. Most of the time the frontend will be the
//! `cargo-marker` CLI. However, `cargo-marker` might also be used as a library for UI
//! tests later down the line.

use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use crate::{config::LintDependencyEntry, ExitStatus};

use self::toolchain::Toolchain;

pub mod driver;
pub mod lints;
pub mod toolchain;

/// Markers configuration for any action that requires lint crates to be available.
///
/// It's assumed that all paths in this struct are absolute paths.
#[derive(Debug)]
pub struct Config {
    /// The base directory used by Marker to fetch and compile lints.
    /// This will default to something like `./target/marker`.
    ///
    /// This should generally be used as a base path for everything. Notable
    /// exceptions can be the installation of a driver or the compilation of
    /// a lint for uitests.
    pub marker_dir: PathBuf,
    /// The list of lints.
    pub lints: HashMap<String, LintDependencyEntry>,
    /// Additional flags, which should be passed to rustc during the compilation
    /// of crates.
    pub build_rustc_flags: String,
    /// Indicates if this is a release or debug build.
    pub debug_build: bool,
    /// Indicates if this is a development build.
    pub dev_build: bool,
    pub toolchain: Toolchain,
}

impl Config {
    pub fn try_base_from(toolchain: Toolchain) -> Result<Self, ExitStatus> {
        Ok(Self {
            marker_dir: toolchain.find_target_dir()?.join("marker"),
            lints: HashMap::default(),
            build_rustc_flags: String::new(),
            debug_build: false,
            dev_build: cfg!(feature = "dev-build"),
            toolchain,
        })
    }

    fn markers_target_dir(&self) -> PathBuf {
        self.marker_dir.join("target")
    }

    fn lint_crate_dir(&self) -> PathBuf {
        self.marker_dir.join("lints")
    }
}

pub fn run_check(config: &Config, additional_cargo_args: &[String]) -> Result<(), ExitStatus> {
    // If this is a dev build, we want to rebuild the driver before checking
    if config.dev_build {
        driver::install_driver(false, true, &config.build_rustc_flags)?;
    }

    println!();
    println!("Compiling Lints:");
    let lints = lints::build_lints(config)?;
    let lint_paths: Vec<_> = lints
        .iter()
        .map(|krate| OsString::from(krate.file.as_os_str()))
        .collect();

    println!();
    println!("Start linting:");

    let mut cmd = config.toolchain.cargo_command();
    cmd.arg("check");
    cmd.args(additional_cargo_args);

    cmd.env("RUSTC_WORKSPACE_WRAPPER", config.toolchain.driver_path.as_os_str());
    cmd.env("MARKER_LINT_CRATES", lint_paths.join(OsStr::new(";")));
    let exit_status = cmd
        .spawn()
        .expect("could not run cargo")
        .wait()
        .expect("failed to wait for cargo?");

    if exit_status.success() {
        Ok(())
    } else {
        Err(ExitStatus::MarkerCheckFailed)
    }
}
