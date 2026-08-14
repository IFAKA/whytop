use crate::{process::ProcessSnapshot, redaction};

pub fn build(snapshot: &ProcessSnapshot, question: &str) -> String {
    let evidence =
        serde_json::to_string_pretty(&redaction::bounded(snapshot)).unwrap_or_else(|_| "{}".into());
    format!("You are a cautious local process analyst. Use only OBSERVED EVIDENCE below. Separate observations from inference, never invent unavailable facts, and say when evidence is insufficient.\n\nOBSERVED EVIDENCE:\n{evidence}\n\nUSER QUESTION:\n{}", question.chars().take(500).collect::<String>())
}

#[cfg(test)]
mod tests {
    #[test]
    fn prompt_mentions_observed_evidence() {
        let p = crate::process::ProcessSnapshot {
            pid: 1,
            start_time: 1,
            parent_pid: None,
            name: "x".into(),
            cpu_percent: 0.,
            memory_bytes: 0,
            executable: None,
            command_line: None,
            parent_name: None,
            child_count: 0,
            file_activity: vec![],
            network_activity: vec![],
            signature: None,
            unavailable: vec![],
        };
        assert!(super::build(&p, "why?").contains("OBSERVED EVIDENCE"));
    }
}
