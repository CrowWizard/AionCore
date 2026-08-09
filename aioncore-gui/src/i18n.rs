use gpui::App;

use crate::AppState;
use crate::app::actions::SelectLocale;
use crate::app::app_menus;
use crate::panels::AppSettings;

pub fn init(cx: &mut App) {
    let locale = AppSettings::global(cx).locale.clone();
    rust_i18n::set_locale(locale.as_ref());

    cx.on_action(|action: &SelectLocale, cx| {
        change_locale(action.0.as_ref());
        AppSettings::global_mut(cx).locale = action.0.clone();
        let title = AppState::global(cx).app_title().clone();
        if !title.is_empty() {
            app_menus::init(title, cx);
        }
        cx.refresh_windows();
    });
}

pub fn change_locale(locale: &str) {
    rust_i18n::set_locale(locale);
}
