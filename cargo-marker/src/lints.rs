use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command,
};

use crate::ExitStatus;

pub struct LintCrateSpec<'a> {
    package_name: Option<&'a str>,
    dir: &'a Path,
}

impl<'a> LintCrateSpec<'a> {
    pub fn new(package_name: Option<&'a str>, dir: &'a Path) -> Self {
        Self { package_name, dir }
    }

    /// Currently only checks for semicolons, can be extended in the future
    pub fn validate(&self) -> bool {
        self.dir.to_string_lossy().contains(';')
    }

    pub fn build_self(&self, target_dir: &Path, verbose: bool) -> Result<PathBuf, ExitStatus> {
        build_local_lint_crate(self, target_dir, verbose)
    }
}

/// This creates a debug build for a local crate. The path of the build library
/// will be returned, if the operation was successful.
pub fn build_local_lint_crate(
    krate: &LintCrateSpec<'_>,
    target_dir: &Path,
    verbose: bool,
) -> Result<PathBuf, ExitStatus> {
    if !krate.dir.exists() {
        eprintln!("The given lint can't be found, searched at: `{}`", krate.dir.display());
        return Err(ExitStatus::LintCrateNotFound);
    }

    // Compile the lint crate
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    if verbose {
        cmd.arg("--verbose");
    }
    if let Some(name) = krate.package_name {
        cmd.arg("--package");
        cmd.arg(name);
    }
    let exit_status = cmd
        .current_dir(std::fs::canonicalize(krate.dir).unwrap())
        .args(["--lib", "--target-dir"])
        .arg(target_dir.as_os_str())
        .env("RUSTFLAGS", "--cap-lints=allow")
        .spawn()
        .expect("could not run cargo")
        .wait()
        .expect("failed to wait for cargo?");

    if !exit_status.success() {
        return Err(ExitStatus::LintCrateBuildFail);
    }

    // Find the final binary and return the string
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let lib_file_prefix = "lib";
    #[cfg(target_os = "windows")]
    let lib_file_prefix = "";

    // FIXME: currently this expect, that the lib name is the same as the crate dir.
    // See marker#60
    let file_name = format!(
        "{lib_file_prefix}{}",
        krate.dir.file_name().and_then(OsStr::to_str).unwrap_or_default()
    );
    // Here `debug` is attached as the crate is build with the `cargo build` command
    let mut krate_path = target_dir.join("debug").join(file_name);

    #[cfg(target_os = "linux")]
    krate_path.set_extension("so");
    #[cfg(target_os = "macos")]
    krate_path.set_extension("dylib");
    #[cfg(target_os = "windows")]
    krate_path.set_extension("dll");

    if !krate_path.exists() && !krate_path.is_file() {
        Err(ExitStatus::LintCrateLibNotFound)
    } else {
        Ok(krate_path)
    }
}
