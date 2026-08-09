use std::path::PathBuf;

use autocorrect::ignorer::Ignorer;
use gpui_component::tree::TreeItem;
use lsp_types::{CompletionItem, CompletionTextEdit, InsertReplaceEdit};

// ============================================================================
// Constants
// ============================================================================

pub const RUST_DOC_URLS: &[(&str, &str)] = &[
    ("String", "string/struct.String"),
    ("Debug", "fmt/trait.Debug"),
    ("Clone", "clone/trait.Clone"),
    ("Option", "option/enum.Option"),
    ("Result", "result/enum.Result"),
    ("Vec", "vec/struct.Vec"),
    ("HashMap", "collections/hash_map/struct.HashMap"),
    ("HashSet", "collections/hash_set/struct.HashSet"),
    ("Arc", "sync/struct.Arc"),
    ("RwLock", "sync/struct.RwLock"),
    ("Duration", "time/struct.Duration"),
];

// ============================================================================
// Helper Functions
// ============================================================================

pub fn completion_item(
    replace_range: &lsp_types::Range,
    label: &str,
    replace_text: &str,
    documentation: &str,
) -> CompletionItem {
    CompletionItem {
        label: label.to_string(),
        kind: Some(lsp_types::CompletionItemKind::FUNCTION),
        text_edit: Some(CompletionTextEdit::InsertAndReplace(InsertReplaceEdit {
            new_text: replace_text.to_string(),
            insert: *replace_range,
            replace: *replace_range,
        })),
        documentation: Some(lsp_types::Documentation::String(documentation.to_string())),
        insert_text: None,
        ..Default::default()
    }
}

pub fn build_file_items(ignorer: &Ignorer, root: &PathBuf, path: &PathBuf) -> Vec<TreeItem> {
    let mut items = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let relative_path = path.strip_prefix(root).unwrap_or(&path);
            if ignorer.is_ignored(&relative_path.to_string_lossy())
                || relative_path.ends_with(".git")
            {
                continue;
            }
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown")
                .to_string();
            let id = path.to_string_lossy().to_string();
            if path.is_dir() {
                let children = build_file_items(ignorer, &root, &path);
                items.push(TreeItem::new(id, file_name).children(children));
            } else {
                items.push(TreeItem::new(id, file_name));
            }
        }
    }
    items.sort_by(|a, b| {
        b.is_folder()
            .cmp(&a.is_folder())
            .then(a.label.cmp(&b.label))
    });
    items
}
