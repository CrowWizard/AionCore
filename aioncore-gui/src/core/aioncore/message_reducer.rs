use std::collections::HashMap;

use serde_json::Value;

use super::models::{ConversationRuntimeSummary, MessageResponse};

#[derive(Clone, Debug, PartialEq)]
pub struct MessageView {
    pub id: String,
    pub conversation_id: String,
    pub turn_id: Option<String>,
    pub kind: String,
    pub content: Value,
    pub position: Option<String>,
    pub status: Option<String>,
    pub hidden: bool,
    pub created_at: Option<i64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MessageStreamState {
    pub messages: Vec<MessageView>,
    pub runtime: Option<ConversationRuntimeSummary>,
    pub active_turn_id: Option<String>,
    pub cancelling: bool,
}

#[derive(Clone, Debug, Default)]
pub struct MessageStreamReducer {
    state: MessageStreamState,
    indexes: HashMap<String, usize>,
}

impl MessageStreamReducer {
    pub fn snapshot(&self) -> MessageStreamState {
        self.state.clone()
    }

    pub fn replace_history(&mut self, messages: &[MessageResponse]) {
        let streamed: HashMap<String, MessageView> = self
            .state
            .messages
            .iter()
            .filter(|message| message.created_at.is_none())
            .cloned()
            .map(|message| (message.id.clone(), message))
            .collect();
        self.state.messages.clear();
        self.indexes.clear();
        for message in messages {
            self.upsert(history_view(message));
        }
        for message in streamed.into_values() {
            if !self.indexes.contains_key(&message.id) {
                self.upsert(message);
            }
        }
    }

    pub fn begin_turn(
        &mut self,
        conversation_id: &str,
        msg_id: String,
        turn_id: String,
        content: String,
        runtime: ConversationRuntimeSummary,
    ) {
        self.state.active_turn_id = Some(turn_id.clone());
        self.state.runtime = Some(runtime);
        self.state.cancelling = false;
        self.upsert(MessageView {
            id: msg_id,
            conversation_id: conversation_id.to_owned(),
            turn_id: Some(turn_id),
            kind: "text".to_owned(),
            content: serde_json::json!({ "content": content }),
            position: Some("right".to_owned()),
            status: Some("finish".to_owned()),
            hidden: false,
            created_at: None,
        });
    }

    pub fn set_cancelling(&mut self, runtime: ConversationRuntimeSummary) {
        self.state.cancelling = runtime.state == "cancelling";
        self.state.runtime = Some(runtime);
    }

    pub fn apply_user_created(&mut self, data: &Value) -> bool {
        let Some(conversation_id) = string(data, "conversation_id") else {
            return false;
        };
        let Some(msg_id) = string(data, "msg_id") else {
            return false;
        };
        self.upsert(MessageView {
            id: msg_id,
            conversation_id,
            turn_id: None,
            kind: "text".to_owned(),
            content: serde_json::json!({ "content": data.get("content").cloned().unwrap_or(Value::Null) }),
            position: string(data, "position"),
            status: string(data, "status"),
            hidden: data.get("hidden").and_then(Value::as_bool).unwrap_or(false),
            created_at: data.get("created_at").and_then(Value::as_i64),
        });
        true
    }

    pub fn apply_stream(&mut self, data: &Value) -> bool {
        let Some(conversation_id) = string(data, "conversation_id") else {
            return false;
        };
        let Some(msg_id) = string(data, "msg_id") else {
            return false;
        };
        let Some(turn_id) = string(data, "turn_id") else {
            return false;
        };
        let Some(kind) = string(data, "type") else {
            return false;
        };
        if !matches!(
            kind.as_str(),
            "text"
                | "content"
                | "thinking"
                | "tool_call"
                | "acp_tool_call"
                | "tool_group"
                | "tips"
                | "error"
                | "finish"
        ) {
            log::debug!("Ignoring unsupported AionCore stream event: type={kind}");
            return false;
        }
        if self.state.active_turn_id.as_deref() == Some(turn_id.as_str()) && matches!(kind.as_str(), "finish" | "error")
        {
            self.state.active_turn_id = None;
            self.state.cancelling = false;
        }
        if kind == "finish" {
            return true;
        }
        self.upsert(MessageView {
            id: msg_id,
            conversation_id,
            turn_id: Some(turn_id),
            kind,
            content: data.get("data").cloned().unwrap_or(Value::Null),
            position: string(data, "position"),
            status: string(data, "status"),
            hidden: data.get("hidden").and_then(Value::as_bool).unwrap_or(false),
            created_at: None,
        });
        true
    }

    fn upsert(&mut self, message: MessageView) {
        if let Some(index) = self.indexes.get(&message.id).copied() {
            self.state.messages[index] = merge_view(self.state.messages[index].clone(), message);
            return;
        }
        let index = self.state.messages.len();
        self.indexes.insert(message.id.clone(), index);
        self.state.messages.push(message);
    }
}

fn history_view(message: &MessageResponse) -> MessageView {
    MessageView {
        id: message.msg_id.clone().unwrap_or_else(|| message.id.clone()),
        conversation_id: message.conversation_id.clone(),
        turn_id: None,
        kind: message.message_type.as_str().unwrap_or("unknown").to_owned(),
        content: message.content.clone(),
        position: message.position.as_ref().and_then(Value::as_str).map(str::to_owned),
        status: message.status.as_ref().and_then(Value::as_str).map(str::to_owned),
        hidden: message.hidden,
        created_at: Some(message.created_at),
    }
}

fn merge_view(mut old: MessageView, new: MessageView) -> MessageView {
    old.conversation_id = new.conversation_id;
    old.turn_id = new.turn_id.or(old.turn_id);
    old.kind = new.kind;
    old.content = merge_content(old.content, new.content);
    old.position = new.position.or(old.position);
    old.status = new.status.or(old.status);
    old.hidden = new.hidden;
    old.created_at = new.created_at.or(old.created_at);
    old
}

fn merge_content(old: Value, new: Value) -> Value {
    let old_text = content_text(&old).map(str::to_owned);
    let new_text = content_text(&new).map(str::to_owned);
    let (Some(old_text), Some(new_text)) = (old_text, new_text) else {
        return merge_json_value(old, new);
    };
    let merged = merge_json_value(old, new);
    if new_text.starts_with(&old_text) {
        return with_content(merged, new_text);
    }
    if old_text.starts_with(&new_text) {
        return with_content(merged, old_text);
    }
    with_content(merged, format!("{old_text}{new_text}"))
}

fn merge_json_value(mut old: Value, new: Value) -> Value {
    let new_object = match new {
        Value::Object(object) => object,
        replacement => return replacement,
    };
    let Value::Object(old_object) = &mut old else {
        return Value::Object(new_object);
    };
    for (key, value) in new_object {
        if value.is_null() {
            continue;
        }
        match old_object.get_mut(&key) {
            Some(existing) => *existing = merge_json_value(existing.clone(), value),
            None => {
                old_object.insert(key, value);
            }
        }
    }
    old
}

fn content_text(value: &Value) -> Option<&str> {
    value.get("content").and_then(Value::as_str).or_else(|| value.as_str())
}

fn with_content(value: Value, content: String) -> Value {
    match value {
        Value::Object(mut object) => {
            object.insert("content".to_owned(), Value::String(content));
            Value::Object(object)
        }
        _ => serde_json::json!({ "content": content }),
    }
}

fn string(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::MessageStreamReducer;

    #[test]
    fn merges_text_deltas_by_message_id() {
        let mut reducer = MessageStreamReducer::default();
        for content in ["hel", "lo"] {
            assert!(reducer.apply_stream(&json!({
                "conversation_id": "conversation-1",
                "msg_id": "message-1",
                "turn_id": "turn-1",
                "type": "content",
                "data": { "content": content },
            })));
        }
        let messages = reducer.snapshot().messages;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content["content"], "hello");
    }

    #[test]
    fn rejects_incomplete_or_unknown_stream_frames() {
        let mut reducer = MessageStreamReducer::default();
        assert!(!reducer.apply_stream(&json!({ "type": "content" })));
        assert!(!reducer.apply_stream(&json!({
            "conversation_id": "conversation-1", "msg_id": "message-1", "turn_id": "turn-1", "type": "ask"
        })));
        assert!(reducer.snapshot().messages.is_empty());
    }

    #[test]
    fn preserves_tool_fields_across_incremental_updates() {
        let mut reducer = MessageStreamReducer::default();
        assert!(reducer.apply_stream(&json!({
            "conversation_id": "conversation-1",
            "msg_id": "tool-1",
            "turn_id": "turn-1",
            "type": "tool_call",
            "data": {
                "call_id": "tool-1",
                "name": "Bash",
                "status": "running",
                "input": { "command": "cargo test" }
            }
        })));
        assert!(reducer.apply_stream(&json!({
            "conversation_id": "conversation-1",
            "msg_id": "tool-1",
            "turn_id": "turn-1",
            "type": "tool_call",
            "data": { "status": "completed", "output": "ok" }
        })));
        let message = &reducer.snapshot().messages[0];
        assert_eq!(message.content["name"], "Bash");
        assert_eq!(message.content["input"]["command"], "cargo test");
        assert_eq!(message.content["status"], "completed");
        assert_eq!(message.content["output"], "ok");
    }

    #[test]
    fn accepts_verified_engineering_message_types() {
        for kind in ["acp_tool_call", "tool_group", "tips"] {
            let mut reducer = MessageStreamReducer::default();
            assert!(reducer.apply_stream(&json!({
                "conversation_id": "conversation-1",
                "msg_id": format!("{kind}-1"),
                "turn_id": "turn-1",
                "type": kind,
                "data": {}
            })));
            assert_eq!(reducer.snapshot().messages[0].kind, kind);
        }
    }

    #[test]
    fn thinking_done_frame_keeps_text_and_adds_completion_metadata() {
        let mut reducer = MessageStreamReducer::default();
        for data in [
            json!({ "content": "Inspecting code" }),
            json!({ "content": "", "status": "done", "duration": 1_500 }),
        ] {
            assert!(reducer.apply_stream(&json!({
                "conversation_id": "conversation-1",
                "msg_id": "thinking-1",
                "turn_id": "turn-1",
                "type": "thinking",
                "data": data
            })));
        }
        let snapshot = reducer.snapshot();
        let content = &snapshot.messages[0].content;
        assert_eq!(content["content"], "Inspecting code");
        assert_eq!(content["status"], "done");
        assert_eq!(content["duration"], 1_500);
    }
}
