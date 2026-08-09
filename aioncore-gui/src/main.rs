#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aioncore_gui::Assets;
use aioncore_gui::core::aioncore::{apply_backend_url_override, load_gui_config};
use aioncore_gui::workspace::open_new;

fn main() {
    let mut config = match load_gui_config() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Failed to load AionCore GUI config: {error}");
            return;
        }
    };

    if let Err(error) = apply_backend_url_override(&mut config) {
        eprintln!("Invalid backend URL configuration: {error}");
        return;
    }

    let app = gpui_platform::application().with_assets(Assets);
    app.run(move |cx| {
        aioncore_gui::init(cx);

        #[cfg(feature = "tray")]
        {
            if let Err(error) = aioncore_gui::system_tray::init_platform() {
                log::error!("Failed to initialize platform for system tray: {error}");
            }

            match aioncore_gui::system_tray::SystemTray::new() {
                Ok(tray) => {
                    aioncore_gui::system_tray::setup_tray_event_handler(tray, cx);
                    log::info!("System tray initialized successfully");
                }
                Err(error) => {
                    log::error!("Failed to initialize system tray: {error}");
                }
            }
        }
        log::info!("AionCore GUI configured for backend {}", config.backend_url);
        if let Err(error) = aioncore_gui::AppState::global_mut(cx).connect_core(&config.backend_url) {
            log::warn!("Failed to initialize AionCore connection: {error}");
        }

        open_new(cx, |_, _, _| {}).detach();
    });
}
