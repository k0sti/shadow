#[cfg(target_os = "linux")]
use std::{
    io,
    path::{Path, PathBuf},
};

#[cfg(target_os = "linux")]
use std::process::Command;

#[cfg(target_os = "linux")]
use std::os::unix::process::CommandExt;
#[cfg(target_os = "linux")]
use url::Url;

#[cfg(target_os = "linux")]
fn main() {
    if let Err(error) = run_linux() {
        eprintln!("shadow-cog-demo: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(target_os = "linux"))]
fn main() {
    println!("shadow-cog-demo: Linux-only demo; nothing to launch here.");
}

#[cfg(target_os = "linux")]
fn run_linux() -> io::Result<()> {
    let index = asset_url("assets/index.html")?;
    let browser = browser_command();
    let error = Command::new(browser)
        .envs(std::env::vars_os())
        .env("GDK_BACKEND", "wayland")
        .arg(index.as_str())
        .exec();
    Err(error)
}

#[cfg(target_os = "linux")]
fn browser_command() -> &'static str {
    // nixpkgs currently ships Grafana's `cog`, not the Igalia WPE launcher,
    // so use an available WebKitGTK browser harness in the guest.
    "epiphany"
}

#[cfg(target_os = "linux")]
fn asset_url(relative_path: impl AsRef<Path>) -> io::Result<Url> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join(relative_path);
    let absolute = path.canonicalize().unwrap_or(path);

    Url::from_file_path(&absolute).map_err(|()| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "could not convert asset path to file URL: {}",
                absolute.display()
            ),
        )
    })
}
