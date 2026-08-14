use gpui::{
    AnyElement, App, ClickEvent, ElementId, IntoElement, ParentElement, SharedString, Styled, Window, div,
    prelude::FluentBuilder as _, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, Sizable,
    button::{Button, ButtonVariants},
    h_flex,
    text::TextView,
    v_flex,
};
use serde_json::Value;

use crate::core::aioncore::MessageView;

pub fn render_message(
    message: &MessageView,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &mut App,
) -> AnyElement {
    match message.kind.as_str() {
        "text" | "content" => render_text_message(message, cx),
        "thinking" => render_thinking(message, expanded, on_toggle, cx),
        "tool_call" | "acp_tool_call" => render_tool_call(message, expanded, on_toggle, cx),
        "tool_group" => render_tool_group(message, cx),
        "error" | "tips" => render_error(message, cx),
        _ => render_fallback(message, cx),
    }
}

fn render_text_message(message: &MessageView, cx: &mut App) -> AnyElement {
    let text = content_text(&message.content);
    if message.position.as_deref() == Some("right") {
        return v_flex()
            .w_full()
            .gap_2()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(Icon::new(IconName::User).size(px(16.)).text_color(cx.theme().accent))
                    .child(
                        div()
                            .text_size(px(13.))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .child("You"),
                    ),
            )
            .child(
                div()
                    .pl_6()
                    .pr_3()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .whitespace_normal()
                    .child(text),
            )
            .into_any_element();
    }

    let markdown_id = SharedString::from(format!("message-{}-markdown", message.id));
    h_flex()
        .w_full()
        .items_start()
        .gap_2()
        .child(
            Icon::new(IconName::Bot)
                .size(px(16.))
                .mt_1()
                .text_color(cx.theme().foreground),
        )
        .child(
            div().w_full().pr_3().child(
                TextView::markdown(markdown_id, text)
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .selectable(true)
                    .pr_3(),
            ),
        )
        .into_any_element()
}

fn render_thinking(
    message: &MessageView,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &mut App,
) -> AnyElement {
    let text = content_text(&message.content);
    let duration = message
        .content
        .get("duration_ms")
        .and_then(Value::as_u64)
        .or_else(|| message.content.get("duration").and_then(Value::as_u64))
        .map(format_duration);
    let complete = message.content.get("status").and_then(Value::as_str) == Some("done")
        || message.status.as_deref() == Some("finish");
    let label = match (complete, duration) {
        (true, Some(duration)) => format!("Thought for {duration}"),
        (true, None) => "Thought complete".to_owned(),
        (false, _) => "Thinking...".to_owned(),
    };

    let mut container = v_flex().w_full().gap_2().pl_6().child(
        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .p_2()
            .rounded(cx.theme().radius)
            .bg(cx.theme().muted.opacity(0.3))
            .child(
                Icon::new(IconName::Bot)
                    .size(px(14.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(label),
            )
            .child(
                Button::new(toggle_id("thinking", &message.id))
                    .icon(if expanded {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .ghost()
                    .xsmall()
                    .on_click(on_toggle),
            ),
    );
    if expanded && !text.is_empty() {
        container = container.child(
            div()
                .pl_6()
                .pr_3()
                .text_sm()
                .italic()
                .text_color(cx.theme().foreground.opacity(0.8))
                .whitespace_normal()
                .child(text),
        );
    }
    container.into_any_element()
}

fn render_tool_call(
    message: &MessageView,
    expanded: bool,
    on_toggle: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &mut App,
) -> AnyElement {
    let tool = ToolCallView::parse(message);
    let status = tool
        .status
        .as_deref()
        .unwrap_or_else(|| message.status.as_deref().unwrap_or("pending"));
    let (status_icon, status_color) = status_presentation(status, cx);
    let has_details = tool.description.is_some() || tool.input.is_some() || tool.output.is_some();

    let mut container = v_flex().w_full().gap_2().pl_6().child(
        h_flex()
            .w_full()
            .items_center()
            .gap_3()
            .p_2()
            .rounded(cx.theme().radius)
            .bg(cx.theme().secondary)
            .child(
                tool_icon(&tool.name)
                    .size(px(16.))
                    .text_color(cx.theme().muted_foreground),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.))
                    .text_size(px(13.))
                    .text_color(cx.theme().foreground)
                    .whitespace_normal()
                    .child(tool.title()),
            )
            .child(status_icon.size(px(14.)).text_color(status_color))
            .when(has_details, |this| {
                this.child(
                    Button::new(toggle_id("tool", &message.id))
                        .icon(if expanded {
                            IconName::ChevronUp
                        } else {
                            IconName::ChevronDown
                        })
                        .ghost()
                        .xsmall()
                        .on_click(on_toggle),
                )
            }),
    );

    if expanded && has_details {
        container = container.child(render_tool_details(&tool, cx));
    }
    container.into_any_element()
}

fn render_tool_details(tool: &ToolCallView, cx: &mut App) -> AnyElement {
    let mut details = v_flex().w_full().gap_2().pl_8().pr_3();
    if let Some(description) = &tool.description {
        details = details.child(
            div()
                .text_size(px(12.))
                .text_color(cx.theme().muted_foreground)
                .whitespace_normal()
                .child(description.clone()),
        );
    }
    if let Some(input) = &tool.input {
        details = details.child(detail_block("Input", input, cx));
    }
    if let Some(output) = &tool.output {
        details = details.child(detail_block("Output", output, cx));
    }
    details.into_any_element()
}

fn detail_block(label: &str, value: &Value, cx: &mut App) -> AnyElement {
    v_flex()
        .w_full()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label.to_owned()),
        )
        .child(
            div()
                .w_full()
                .max_h(px(240.))
                .overflow_hidden()
                .p_2()
                .rounded(cx.theme().radius)
                .bg(cx.theme().muted.opacity(0.35))
                .font_family(cx.theme().mono_font_family.clone())
                .text_size(px(12.))
                .line_height(px(18.))
                .text_color(cx.theme().foreground)
                .whitespace_normal()
                .child(value_preview(value, 12)),
        )
        .into_any_element()
}

fn render_tool_group(message: &MessageView, cx: &mut App) -> AnyElement {
    let entries = message.content.as_array().cloned().unwrap_or_default();
    v_flex()
        .w_full()
        .gap_1()
        .pl_6()
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(cx.theme().muted_foreground)
                .child("Agent activity"),
        )
        .children(entries.into_iter().map(|entry| {
            let name = field_string(&entry, &["name"]).unwrap_or_else(|| "Agent task".to_owned());
            let status = field_string(&entry, &["status"]).unwrap_or_else(|| "Pending".to_owned());
            let description = field_string(&entry, &["description"]);
            let (icon, color) = status_presentation(&status, cx);
            h_flex()
                .w_full()
                .items_center()
                .gap_2()
                .p_2()
                .rounded(cx.theme().radius)
                .bg(cx.theme().secondary)
                .child(icon.size(px(14.)).text_color(color))
                .child(div().text_sm().font_weight(gpui::FontWeight::MEDIUM).child(name))
                .when_some(description, |this, description| {
                    this.child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(description),
                    )
                })
        }))
        .into_any_element()
}

fn render_error(message: &MessageView, cx: &mut App) -> AnyElement {
    h_flex()
        .w_full()
        .items_start()
        .gap_2()
        .pl_6()
        .child(
            Icon::new(IconName::CircleX)
                .size(px(15.))
                .mt_1()
                .text_color(cx.theme().red),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().foreground)
                .whitespace_normal()
                .child(content_text(&message.content)),
        )
        .into_any_element()
}

fn render_fallback(message: &MessageView, cx: &mut App) -> AnyElement {
    v_flex()
        .w_full()
        .gap_1()
        .pl_6()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(message.kind.clone()),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().foreground)
                .whitespace_normal()
                .child(content_text(&message.content)),
        )
        .into_any_element()
}

#[derive(Debug, Default, PartialEq)]
struct ToolCallView {
    name: String,
    status: Option<String>,
    description: Option<String>,
    input: Option<Value>,
    output: Option<Value>,
}

impl ToolCallView {
    fn parse(message: &MessageView) -> Self {
        let source = if message.kind == "acp_tool_call" {
            message.content.get("update").unwrap_or(&message.content)
        } else {
            &message.content
        };
        let input = source
            .get("input")
            .or_else(|| source.get("args"))
            .or_else(|| source.get("raw_input"))
            .or_else(|| source.get("rawInput"))
            .filter(|value| !value.is_null())
            .cloned();
        let output = source
            .get("output")
            .or_else(|| source.get("raw_output"))
            .or_else(|| source.get("rawOutput"))
            .filter(|value| !value.is_null())
            .cloned()
            .or_else(|| acp_content_output(source));
        Self {
            name: field_string(source, &["name", "title"]).unwrap_or_else(|| "Tool".to_owned()),
            status: field_string(source, &["status"]),
            description: field_string(source, &["description"]),
            input,
            output,
        }
    }

    fn title(&self) -> String {
        self.description.clone().unwrap_or_else(|| self.name.clone())
    }
}

fn acp_content_output(source: &Value) -> Option<Value> {
    let items = source.get("content")?.as_array()?;
    let text = items
        .iter()
        .filter_map(|item| item.pointer("/content/text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then(|| Value::String(text))
}

fn content_text(value: &Value) -> String {
    value
        .get("content")
        .and_then(Value::as_str)
        .or_else(|| value.get("message").and_then(Value::as_str))
        .or_else(|| value.get("error").and_then(Value::as_str))
        .or_else(|| value.as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| value_preview(value, 12))
}

fn field_string(value: &Value, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| value.get(*name).and_then(Value::as_str))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn value_preview(value: &Value, max_lines: usize) -> String {
    let text = match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string_pretty(value).unwrap_or_default(),
    };
    text.lines().take(max_lines).collect::<Vec<_>>().join("\n")
}

fn format_duration(duration_ms: u64) -> String {
    if duration_ms < 1_000 {
        return "<1s".to_owned();
    }
    format!("{}s", duration_ms / 1_000)
}

fn status_presentation(status: &str, cx: &App) -> (Icon, gpui::Hsla) {
    match status.to_ascii_lowercase().as_str() {
        "completed" | "success" | "finish" | "done" => (Icon::new(IconName::CircleCheck), cx.theme().green),
        "failed" | "error" => (Icon::new(IconName::CircleX), cx.theme().red),
        "canceled" | "cancelled" => (Icon::new(IconName::CircleX), cx.theme().muted_foreground),
        "running" | "executing" | "in_progress" | "work" => (Icon::new(IconName::LoaderCircle), cx.theme().accent),
        _ => (Icon::new(IconName::Dash), cx.theme().muted_foreground),
    }
}

fn tool_icon(name: &str) -> Icon {
    let name = name.to_ascii_lowercase();
    if name.contains("read") {
        Icon::new(IconName::Eye)
    } else if name.contains("edit") || name.contains("write") {
        Icon::new(IconName::Replace)
    } else if name.contains("delete") {
        Icon::new(IconName::Delete)
    } else if name.contains("search") || name.contains("glob") || name.contains("grep") {
        Icon::new(IconName::Search)
    } else if name.contains("bash") || name.contains("terminal") || name.contains("command") {
        Icon::new(IconName::SquareTerminal)
    } else {
        Icon::new(IconName::Ellipsis)
    }
}

fn toggle_id(prefix: &str, message_id: &str) -> ElementId {
    SharedString::from(format!("{prefix}-{message_id}-toggle")).into()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::ToolCallView;
    use crate::core::aioncore::MessageView;

    fn message(kind: &str, content: serde_json::Value) -> MessageView {
        MessageView {
            id: "message-1".into(),
            conversation_id: "conversation-1".into(),
            turn_id: None,
            kind: kind.into(),
            content,
            position: Some("left".into()),
            status: None,
            hidden: false,
            created_at: None,
        }
    }

    #[test]
    fn parses_native_tool_call_contract() {
        let parsed = ToolCallView::parse(&message(
            "tool_call",
            json!({
                "call_id": "call-1",
                "name": "Bash",
                "status": "completed",
                "input": { "command": "cargo test" },
                "output": "ok",
                "description": "Run tests"
            }),
        ));
        assert_eq!(parsed.name, "Bash");
        assert_eq!(parsed.status.as_deref(), Some("completed"));
        assert_eq!(parsed.input, Some(json!({ "command": "cargo test" })));
        assert_eq!(parsed.output, Some(json!("ok")));
        assert_eq!(parsed.title(), "Run tests");
    }

    #[test]
    fn parses_acp_tool_call_without_converting_to_acp_schema() {
        let parsed = ToolCallView::parse(&message(
            "acp_tool_call",
            json!({
                "session_id": "session-1",
                "update": {
                    "session_update": "tool_call",
                    "tool_call_id": "call-1",
                    "title": "Search",
                    "status": "in_progress",
                    "raw_input": { "pattern": "needle" },
                    "content": [{ "type": "content", "content": { "type": "text", "text": "match" } }]
                }
            }),
        ));
        assert_eq!(parsed.name, "Search");
        assert_eq!(parsed.status.as_deref(), Some("in_progress"));
        assert_eq!(parsed.input, Some(json!({ "pattern": "needle" })));
        assert_eq!(parsed.output, Some(json!("match")));
    }
}
