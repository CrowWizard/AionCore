#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::panic;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use aioncore_gui::Assets;
use aioncore_gui::core::aioncore::{apply_backend_url_override, load_gui_config};
use aioncore_gui::workspace::open_new;

static STARTUP_LOG: OnceLock<Mutex<File>> = OnceLock::new();

fn main() {
    init_startup_diagnostics();
    write_startup_log("process.start");

    let mut config = match load_gui_config() {
        Ok(config) => config,
        Err(error) => {
            write_startup_log(&format!("config.load.failed error={error}"));
            eprintln!("Failed to load AionCore GUI config: {error}");
            return;
        }
    };
    write_startup_log("config.load.completed");

    if let Err(error) = apply_backend_url_override(&mut config) {
        write_startup_log(&format!("backend_url.validate.failed error={error}"));
        eprintln!("Invalid backend URL configuration: {error}");
        return;
    }
    write_startup_log("backend_url.validate.completed");

    let app = gpui_platform::application().with_assets(Assets);
    write_startup_log("gpui.application.created");
    app.run(move |cx| {
        write_startup_log("gpui.run.entered");
        aioncore_gui::init(cx);
        write_startup_log("gui.init.completed");

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
        let proxy = aioncore_gui::AppSettings::global(cx).clone();
        let (http_proxy_url, https_proxy_url, all_proxy_url) = if proxy.proxy_enabled {
            (
                Some(proxy.http_proxy_url.as_str()),
                Some(proxy.https_proxy_url.as_str()),
                Some(proxy.all_proxy_url.as_str()),
            )
        } else {
            (None, None, None)
        };
        if let Err(error) = aioncore_gui::AppState::global_mut(cx).connect_core(
            &config.backend_url,
            http_proxy_url,
            https_proxy_url,
            all_proxy_url,
        ) {
            write_startup_log(&format!("backend.connect.failed error={error}"));
            log::warn!("Failed to initialize AionCore connection: {error}");
        } else {
            write_startup_log("backend.connect.started");
        }

        write_startup_log("workspace.open.started");
        open_new(cx, |_, _, _| {
            write_startup_log("workspace.open.completed");
        })
        .detach();
    });
}

fn init_startup_diagnostics() {
    let path = startup_log_path();
    let file = path
        .parent()
        .ok_or_else(|| std::io::Error::other("startup log path has no parent"))
        .and_then(|parent| {
            std::fs::create_dir_all(parent)?;
            OpenOptions::new().create(true).append(true).open(&path)
        });

    match file {
        Ok(file) => {
            let _ = STARTUP_LOG.set(Mutex::new(file));
            write_startup_log(&format!("diagnostics.ready path={}", path.display()));
        }
        Err(error) => eprintln!("Failed to open startup log at {}: {error}", path.display()),
    }

    panic::set_hook(Box::new(|panic_info| {
        write_startup_log(&format!("panic.detected {panic_info}"));
        write_startup_log(&format!(
            "panic.backtrace {}",
            std::backtrace::Backtrace::force_capture()
        ));
    }));
}

fn startup_log_path() -> PathBuf {
    let base_dir = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
    base_dir.join("aioncore-gui").join("logs").join("startup.log")
}

fn write_startup_log(message: &str) {
    let Some(file) = STARTUP_LOG.get() else {
        return;
    };
    let Ok(mut file) = file.lock() else {
        return;
    };

    let _ = writeln!(file, "{} {message}", chrono::Utc::now().to_rfc3339());
    let _ = file.flush();
}
