use std::collections::BTreeMap;

use gpui::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};
use gpui_component::{ActiveTheme, Icon, IconName, h_flex, v_flex};
use serde_json::Value;
use similar::{ChangeTag, TextDiff};

use super::diff_view::{DiffSpec, DiffView};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiffSummaryFile {
    pub diff: DiffSpec,
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DiffSummaryData {
    pub files: Vec<DiffSummaryFile>,
}

impl DiffSummaryData {
    pub fn from_value(value: &Value) -> Self {
        let mut diffs = BTreeMap::<String, DiffSpec>::new();
        collect_diffs(value, &mut diffs);
        let files = diffs
            .into_values()
            .map(|diff| {
                let (additions, deletions) = count_changes(&diff);
                DiffSummaryFile {
                    diff,
                    additions,
                    deletions,
                }
            })
            .collect();
        Self { files }
    }

    pub fn total_additions(&self) -> usize {
        self.files.iter().map(|file| file.additions).sum()
    }
    pub fn total_deletions(&self) -> usize {
        self.files.iter().map(|file| file.deletions).sum()
    }
}

fn collect_diffs(value: &Value, output: &mut BTreeMap<String, DiffSpec>) {
    if let Some(diff_value) = value.get("diff") {
        if let Some(diff) = DiffSpec::from_value(diff_value) {
            output.insert(diff.path.clone(), diff);
        }
    }
    if let Some(diff) = DiffSpec::from_value(value) {
        output.insert(diff.path.clone(), diff);
    }
    match value {
        Value::Object(object) => object.values().for_each(|value| collect_diffs(value, output)),
        Value::Array(values) => values.iter().for_each(|value| collect_diffs(value, output)),
        _ => {}
    }
}

fn count_changes(diff: &DiffSpec) -> (usize, usize) {
    let Some(old_text) = diff.old_text.as_deref() else {
        return (diff.new_text.lines().count(), 0);
    };
    TextDiff::from_lines(old_text, &diff.new_text)
        .iter_all_changes()
        .fold((0, 0), |(additions, deletions), change| match change.tag() {
            ChangeTag::Insert => (additions + 1, deletions),
            ChangeTag::Delete => (additions, deletions + 1),
            ChangeTag::Equal => (additions, deletions),
        })
}

#[derive(Clone, Debug)]
pub struct DiffSummary {
    pub data: DiffSummaryData,
}

impl DiffSummary {
    pub fn from_value(value: &Value) -> Option<Self> {
        let data = DiffSummaryData::from_value(value);
        (!data.files.is_empty()).then_some(Self { data })
    }
}

impl RenderOnce for DiffSummary {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let additions = self.data.total_additions();
        let deletions = self.data.total_deletions();
        v_flex()
            .w_full()
            .gap_1()
            .p_2()
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().secondary)
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::File).size(px(14.)).text_color(cx.theme().accent))
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .child(format!("{} file(s) changed", self.data.files.len())),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().green)
                            .child(format!("+{additions}")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().red)
                            .child(format!("-{deletions}")),
                    ),
            )
            .children(self.data.files.into_iter().map(|file| {
                DiffView::new(file.diff)
                    .max_lines(80)
                    .render(window, cx)
                    .into_any_element()
            }))
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn summarizes_multiple_tool_diffs() {
        let summary = DiffSummaryData::from_value(&json!({"items":[
            {"path":"a.rs","old_text":"a\n","new_text":"a\nb\n"},
            {"path":"b.rs","old_text":"x\ny\n","new_text":"x\n"}
        ]}));
        assert_eq!(summary.files.len(), 2);
        assert_eq!(summary.total_additions(), 1);
        assert_eq!(summary.total_deletions(), 1);
    }
}
