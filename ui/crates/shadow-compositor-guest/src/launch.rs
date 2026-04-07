use std::{
    io,
    path::{Path, PathBuf},
    process::{Child, Command},
};

use shadow_ui_core::{
    app::{find_app, AppId},
    control,
};

use crate::ShadowGuestCompositor;

pub fn launch_app(state: &mut ShadowGuestCompositor, app_id: AppId) -> io::Result<Child> {
    let Some(app) = find_app(app_id) else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "unknown demo app"));
    };
    let runtime_bundle_path = std::env::var(app.runtime_bundle_env).map_err(|_| {
        io::Error::new(
            io::ErrorKind::NotFound,
            format!("missing runtime bundle env {}", app.runtime_bundle_env),
        )
    })?;

    let client_path = std::env::var("SHADOW_APP_CLIENT")
        .or_else(|_| std::env::var("SHADOW_GUEST_CLIENT"))
        .unwrap_or_else(|_| crate::default_guest_client_path());
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| "/data/local/tmp/shadow-runtime".into());

    let mut command = Command::new(&client_path);
    command
        .env("XDG_RUNTIME_DIR", runtime_dir)
        .env(
            control::COMPOSITOR_CONTROL_ENV,
            state.control_socket_path.as_os_str(),
        )
        .env("SHADOW_BLITZ_APP_TITLE", app.window_title)
        .env("SHADOW_BLITZ_WAYLAND_APP_ID", app.wayland_app_id)
        .env("SHADOW_RUNTIME_APP_BUNDLE_PATH", runtime_bundle_path);

    if let Some(value) = std::env::var_os("SHADOW_RUNTIME_HOST_BINARY_PATH") {
        command.env("SHADOW_RUNTIME_HOST_BINARY_PATH", value);
    }
    if let Some(value) = std::env::var("SHADOW_GUEST_CLIENT_ENV").ok() {
        for assignment in value.split_whitespace() {
            if let Some((key, env_value)) = assignment.split_once('=') {
                if !key.is_empty() {
                    command.env(key, env_value);
                }
            }
        }
    }
    if let Some(value) = std::env::var_os("SHADOW_GUEST_CLIENT_EXIT_ON_CONFIGURE") {
        command.env("SHADOW_GUEST_CLIENT_EXIT_ON_CONFIGURE", value);
    }
    if let Some(value) = std::env::var_os("SHADOW_GUEST_CLIENT_LINGER_MS") {
        command.env("SHADOW_GUEST_CLIENT_LINGER_MS", value);
    }

    state.spawn_wayland_command(command, &client_path)
}

#[allow(dead_code)]
fn sibling_binary_path(name: &str) -> Option<PathBuf> {
    let current = std::env::current_exe().ok()?;
    Some(current.with_file_name(name))
}

#[allow(dead_code)]
fn workspace_manifest() -> Option<PathBuf> {
    let mut roots = Vec::new();
    roots.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    if let Ok(current) = std::env::current_dir() {
        roots.push(current);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            roots.push(parent.to_path_buf());
        }
    }

    for root in roots {
        if let Some(manifest) = find_manifest_upwards(&root) {
            return Some(manifest);
        }
    }

    None
}

#[allow(dead_code)]
fn find_manifest_upwards(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        let manifest = ancestor.join("Cargo.toml");
        if manifest.exists() {
            let contents = std::fs::read_to_string(&manifest).ok()?;
            if contents.contains("[workspace]") {
                return Some(manifest);
            }
        }
    }

    None
}
