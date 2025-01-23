use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::LazyLock as Lazy};

pub const DEFAULT_THRESHOLD: f32 = -20.0;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
  pub threshold: f32,
  pub input_device_name: String,
  pub output_device_name: String,
  pub window_size: Option<[f32; 2]>,
}

impl Default for Config {
  fn default() -> Self {
    Self {
      threshold: DEFAULT_THRESHOLD,
      input_device_name: "".to_string(),
      output_device_name: "".to_string(),
      window_size: None,
    }
  }
}

pub static CONFIG_DIRS: Lazy<PathBuf> = Lazy::new(|| {
  ProjectDirs::from("com", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_NAME"))
    .expect("Could not find project dirs")
    .config_dir()
    .to_path_buf()
});

pub static CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| CONFIG_DIRS.join("default.toml"));
