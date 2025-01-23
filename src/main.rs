#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // Hide console window on Windows in release mode

mod compressor;
mod gui;
mod streaming;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  gui::run()?;
  Ok(())
}
