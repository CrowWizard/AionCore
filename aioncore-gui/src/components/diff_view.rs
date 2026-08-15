use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};
use gpui_component::{ActiveTheme, Icon, IconName, h_flex, v_flex};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiffSpec {
    pub path: String,
    pub old_text: Option<String>,
    pub new_text: String,
}

impl DiffSpec {
    pub fn from_value(value: &Value) -> Option<Self> {
        let path = value
            .get("path")
            .or_else(|| value.get("file_path"))
            .and_then(Value::as_str)?;
        let new_text = value
            .get("new_text")
            .or_else(|| value.get("newText"))
            .or_else(|| value.get("content"))
            .and_then(Value::as_str)?;
        Some(Self {
            path: path.to_owned(),
            old_text: value
                .get("old_text")
                .or_else(|| value.get("oldText"))
                .and_then(Value::as_str)
                .map(str::to_owned),
            new_text: new_text.to_owned(),
        })
    }
}

#[derive(Clone, Debug)]
pub struct DiffView {
    diff: DiffSpec,
    max_lines: usize,
    context_lines: usize,
}

impl DiffView {
    pub fn new(diff: DiffSpec) -> Self {
        Self {
            diff,
            max_lines: 500,
            context_lines: 3,
        }
    }

    pub fn max_lines(mut self, max_lines: usize) -> Self {
        self.max_lines = max_lines;
        self
    }

    pub fn context_lines(mut self, context_lines: usize) -> Self {
        self.context_lines = context_lines;
        self
    }

    fn render_lines(&self, cx: &App) -> Vec<AnyElement> {
        let Some(old_text) = self.diff.old_text.as_deref() else {
            return self
                .diff
                .new_text
                .lines()
                .enumerate()
                .take(self.max_lines)
                .map(|(index, line)| render_line('+', "", index + 1, line, cx.theme().green, cx))
                .collect();
        };
        let mut old_line = 1usize;
        let mut new_line = 1usize;
        let mut rows = Vec::new();
        let _ = self.context_lines;
        for change in TextDiff::from_lines(old_text, &self.diff.new_text).iter_all_changes() {
            if rows.len() >= self.max_lines {
                break;
            }
            let line = change.value().trim_end_matches('\n');
            match change.tag() {
                ChangeTag::Equal => {
                    rows.push(render_line(
                        ' ',
                        &format!("{old_line:>4} {new_line:>4}"),
                        new_line,
                        line,
                        cx.theme().foreground,
                        cx,
                    ));
                    old_line += 1;
                    new_line += 1;
                }
                ChangeTag::Delete => {
                    rows.push(render_line(
                        '-',
                        &format!("{old_line:>4}     "),
                        old_line,
                        line,
                        cx.theme().red,
                        cx,
                    ));
                    old_line += 1;
                }
                ChangeTag::Insert => {
                    rows.push(render_line(
                        '+',
                        &format!("     {new_line:>4}"),
                        new_line,
                        line,
                        cx.theme().green,
                        cx,
                    ));
                    new_line += 1;
                }
            }
        }
        rows
    }
}

fn render_line(
    prefix: char,
    numbers: &str,
    _line_number: usize,
    line: &str,
    color: gpui::Hsla,
    cx: &App,
) -> AnyElement {
    h_flex()
        .w_full()
        .font_family(cx.theme().mono_font_family.clone())
        .text_size(px(11.))
        .line_height(px(17.))
        .bg(color.opacity(if prefix == ' ' { 0.0 } else { 0.12 }))
        .child(
            div()
                .min_w(px(105.))
                .px_2()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{numbers} {prefix}")),
        )
        .child(div().flex_1().px_2().text_color(color).child(line.to_owned()))
        .into_any_element()
}

impl RenderOnce for DiffView {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let changed = self.diff.old_text.as_deref() != Some(self.diff.new_text.as_str());
        v_flex()
            .w_full()
            .gap_1()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .child(Icon::new(IconName::File).size(px(14.)).text_color(cx.theme().accent))
                    .child(div().flex_1().text_xs().child(self.diff.path.clone()))
                    .child(if self.diff.old_text.is_none() {
                        "NEW"
                    } else if changed {
                        "CHANGED"
                    } else {
                        "UNCHANGED"
                    }),
            )
            .child(v_flex().w_full().overflow_hidden().children(self.render_lines(cx)))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_native_diff_payload() {
        let diff = DiffSpec::from_value(&json!({"path":"src/lib.rs","old_text":"old","new_text":"new"})).unwrap();
        assert_eq!(diff.path, "src/lib.rs");
        assert_eq!(diff.old_text.as_deref(), Some("old"));
    }
}
