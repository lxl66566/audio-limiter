use crate::config::{Config, CONFIG_PATH, DEFAULT_THRESHOLD};
use crate::streaming::{create_stream, get_devices};
use atomic_float::AtomicF32;
use config_file2::{LoadConfigFile, StoreConfigFile};
use cpal::Stream;
use cpal::{traits::DeviceTrait, Device};
use eframe::egui::{
  self, FontDefinitions, FontFamily, InnerResponse, Layout, Ui, Vec2, ViewportBuilder,
};
use eframe::emath::Align;
use std::path::Path;
use std::sync::atomic::Ordering;
use sys_locale::get_locale;

pub const DEFAULT_ATTACK: f32 = 25.0;
pub const DEFAULT_RELEASE: f32 = 50.0;
pub const PIXELS_PER_POINT: f32 = 2.0;

pub static CURR_THRESHOLD: AtomicF32 = AtomicF32::new(DEFAULT_THRESHOLD);

struct AppData {
  devices: Vec<Device>,
  config: Config,
  input_device_idx: Option<usize>,
  output_device_idx: Option<usize>,
  threshold: f32,
  running: bool,
  input_stream: Option<Stream>,
  output_stream: Option<Stream>,
}

fn get_device_name(devices: &[Device], idx: Option<usize>) -> String {
  idx.map_or_else(
    || "No Device Selected".to_string(),
    |idx| {
      devices[idx]
        .name()
        .unwrap_or_else(|_| "Unknown Device".to_string())
    },
  )
}

fn create_combo_box(
  ui: &mut Ui,
  label: &'static str,
  devices: &[Device],
  device_idx: &mut Option<usize>,
) -> InnerResponse<Option<()>> {
  let device_name = get_device_name(devices, *device_idx);

  ui.label(label);

  let combo = egui::ComboBox::from_id_salt(label)
    .width(ui.available_width() - 7.0)
    .selected_text(device_name)
    .show_ui(ui, |ui| {
      for (i, d) in devices.iter().enumerate() {
        let device_name = d.name().unwrap_or_else(|_| "Unknown Device".to_string());

        ui.selectable_value(device_idx, Some(i), device_name);
      }
    });

  ui.end_row();

  combo
}

impl AppData {
  fn start_stream(&mut self) -> Option<bool> {
    let input_device_idx = self.input_device_idx?;
    let output_device_idx = self.output_device_idx?;

    let input_device = &self.devices[input_device_idx];
    let output_device = &self.devices[output_device_idx];

    let streams = create_stream(input_device, output_device, self.threshold)?;

    self.input_stream = Some(streams.0);
    self.output_stream = Some(streams.1);

    Some(true)
  }

  fn draw_start_stop_button(&mut self, ui: &mut Ui, ctx: &egui::Context) {
    let button_text = if self.running { "Stop" } else { "Start" };

    if ui.button(button_text).clicked() {
      _ = self.store_config(ctx);
      if self.running {
        self.input_stream = None;
        self.output_stream = None;

        self.running = false;
      } else {
        self.running = self.start_stream().unwrap_or(false);
      }
    }

    if self.running {
      CURR_THRESHOLD.store(self.threshold, Ordering::SeqCst);
    }
  }

  fn draw_interface(&mut self, ui: &mut Ui) {
    create_combo_box(
      ui,
      "Input Device",
      &self.devices,
      &mut self.input_device_idx,
    );
    create_combo_box(
      ui,
      "Output Device",
      &self.devices,
      &mut self.output_device_idx,
    );
    ui.label("Threshold");

    ui.add(egui::Slider::new(&mut self.threshold, -200.0..=0.0).max_decimals(0));
    ui.end_row();
    if ui.button("🔄 Refresh devices").clicked() {
      self.devices = get_devices();
      self.input_device_idx = get_device_idx_by_name(&self.devices, &self.config.input_device_name);
      self.output_device_idx =
        get_device_idx_by_name(&self.devices, &self.config.output_device_name);
    }
  }

  /// Collect the config, and store it to the config file
  fn store_config(&mut self, ctx: &egui::Context) -> Result<(), config_file2::error::Error> {
    self.config.input_device_name = get_device_name(&self.devices, self.input_device_idx);
    self.config.output_device_name = get_device_name(&self.devices, self.output_device_idx);
    self.config.threshold = self.threshold;
    let size = ctx.screen_rect().size() * PIXELS_PER_POINT;
    self.config.window_size = Some([size.x, size.y]);

    self.config.store(CONFIG_PATH.as_path())?;
    Ok(())
  }
}

impl eframe::App for AppData {
  fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ctx, |ui| {
      ui.spacing_mut().slider_width = (ui.available_width() - 175.0).max(3.0);
      egui::ScrollArea::vertical().show(ui, |ui| {
        ui.with_layout(Layout::top_down_justified(Align::default()), |ui| {
          egui::Grid::new("app_grid")
            .num_columns(2)
            .spacing([10.0, 10.0])
            .min_col_width(100.0)
            .show(ui, |ui| {
              self.draw_interface(ui);
              ui.with_layout(Layout::right_to_left(Align::default()), |ui| {
                self.draw_start_stop_button(ui, ctx);
              });
            });
        });
      });
    });
  }
}

fn get_device_idx_by_name(devices: &[Device], name: &str) -> Option<usize> {
  devices
    .iter()
    .position(|x| matches!(x.name(), Ok(device_name) if device_name == name))
}

pub fn run() -> Result<(), eframe::Error> {
  let config = Config::load_or_default(CONFIG_PATH.as_path()).unwrap_or_default();
  let devices = get_devices();
  let input_device_idx = get_device_idx_by_name(&devices, &config.input_device_name);
  let output_device_idx = get_device_idx_by_name(&devices, &config.output_device_name);
  let threshold = config.threshold;
  let resized_viewport = config
    .window_size
    .map(|size| ViewportBuilder::default().with_inner_size(Vec2::new(size[0], size[1])))
    .unwrap_or_else(ViewportBuilder::default);

  eframe::run_native(
    "Audio Limiter",
    eframe::NativeOptions {
      viewport: resized_viewport,
      ..Default::default()
    },
    Box::new(|cc: &eframe::CreationContext<'_>| {
      cc.egui_ctx.set_pixels_per_point(PIXELS_PER_POINT);

      // The default font is not supported for Chinese, so we set the font to support Chinese on which the system's locale is zh
      let locale = get_locale().unwrap_or_default();
      if locale.starts_with("zh") {
        let mut fonts = FontDefinitions::default();
        let font_folder = Path::new("C:\\Windows\\Fonts");
        let font_tries = ["msyh.ttc", "simsun.ttc", "simhei.ttf"];
        for font_name in font_tries {
          let font_path = font_folder.join(font_name);
          if let Ok(font_data) = std::fs::read(font_path) {
            fonts.font_data.insert(
              font_name.to_string(),
              egui::FontData::from_owned(font_data).into(),
            );
            if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
              family.insert(0, font_name.to_string());
              break;
            }
          }
        }
        cc.egui_ctx.set_fonts(fonts);
      }

      let app_data = AppData {
        devices,
        config,
        input_device_idx,
        output_device_idx,
        threshold,
        running: false,
        input_stream: None,
        output_stream: None,
      };

      Ok(Box::new(app_data))
    }),
  )
}
