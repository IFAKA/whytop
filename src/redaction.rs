use crate::process::ProcessSnapshot;

pub fn bounded(snapshot: &ProcessSnapshot) -> ProcessSnapshot {
    let mut result = snapshot.clone();
    result.command_line = result.command_line.map(|x| x.chars().take(256).collect());
    result.executable = result.executable.map(|x| x.chars().take(512).collect());
    result.file_activity.truncate(20);
    result.network_activity.truncate(20);
    result
        .unavailable
        .push("environment variables and secrets are intentionally omitted".into());
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redaction_bounds_and_marks_omissions() {
        let mut p = crate::process::ProcessSnapshot {
            pid: 1,
            start_time: 1,
            parent_pid: None,
            name: "a".into(),
            cpu_percent: 0.,
            memory_bytes: 0,
            executable: None,
            command_line: Some("x".repeat(500)),
            parent_name: None,
            child_count: 0,
            file_activity: vec!["f".into(); 30],
            network_activity: vec![],
            signature: None,
            unavailable: vec![],
        };
        p = bounded(&p);
        assert_eq!(p.command_line.unwrap().len(), 256);
        assert_eq!(p.file_activity.len(), 20);
        assert!(p.unavailable.iter().any(|x| x.contains("secrets")));
    }
}
