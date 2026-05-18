use serde::Serialize;

/// Installation mode determined at compile time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallMode {
    Installed,
    Portable,
}

/// Return the compile-time install mode.
///
/// When the `portable` feature is enabled the binary is a portable build;
/// otherwise it is an installed (system) build.
#[tauri::command]
pub const fn install_mode() -> InstallMode {
    if cfg!(feature = "portable") {
        InstallMode::Portable
    } else {
        InstallMode::Installed
    }
}
