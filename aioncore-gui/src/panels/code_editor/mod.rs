mod lsp_providers;
mod lsp_store;
mod panel;
mod types;

pub use panel::CodeEditorPanel;

use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};

pub fn init() {
    LanguageRegistry::singleton().register(
        "navi",
        &LanguageConfig::new(
            "navi",
            tree_sitter_navi::LANGUAGE.into(),
            vec![],
            tree_sitter_navi::HIGHLIGHTS_QUERY,
            "",
            "",
        ),
    );
}
