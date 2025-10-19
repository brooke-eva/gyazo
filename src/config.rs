use std::{fs, path::PathBuf};

use crate::Result;

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Config {
    /// Extracted `Gyazo_session` cookie for internal APIs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cookie: Option<String>,
    /// One of the "linked device" IDs
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
    /// An app's access token
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Upload::is_default")]
    pub upload: Upload,
}

// fn yes() -> bool {
//     true
// }

#[derive(Clone, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct Upload {
    // #[serde(default = "yes")]
    // pub public_access: bool,
    #[serde(default)]
    pub public_metadata: bool,
}

impl Upload {
    fn is_default(&self) -> bool {
        *self == Default::default()
    }
}

impl Config {
    pub fn load() -> Self {
        fs::read_to_string(Self::path())
            .ok()
            .and_then(|contents| toml::from_str(&contents).unwrap())
            .unwrap_or_default()
    }

    pub fn store(&self) -> Result<()> {
        Self::ensure_dir();
        fs::write(Self::path(), toml::to_string_pretty(self).unwrap()).unwrap();
        Ok(())
    }

    // pub fn is_linked(&self) -> bool {
    //     self.device.is_some()
    // }

    // https://docs.rs/dirs/latest/dirs/fn.config_dir.html
    // https://codeberg.org/dirs/dirs-rs/src/branch/main/src/lib.rs
    pub fn dir() -> PathBuf {
        dirs::config_dir().unwrap()
    }

    // #[cfg(target_os = "macos")]
    // pub fn dir() -> PathBuf {
    //     let mut path = std::env::home_dir().unwrap();
    //     path.push("Library");
    //     path.push("Application Support");
    //     path
    // }
    //
    // #[cfg(target_os = "windows")]
    // pub fn dir() -> PathBuf {
    //     let mut path = std::env::home_dir().unwrap();
    //     path.push("AppData");
    //     path.push("Roaming");
    //     path
    // }
    //
    // #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    // pub fn dir() -> PathBuf {
    //     let mut path = std::env::home_dir().unwrap();
    //     path.push(".config");
    //     path
    // }

    fn ensure_dir() {
        fs::create_dir_all(Self::dir()).ok();
    }

    pub fn path() -> PathBuf {
        let mut path = Self::dir();
        path.push("gyazo.toml");
        path
    }
}
