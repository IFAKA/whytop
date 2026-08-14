use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use sysinfo::{Pid, ProcessesToUpdate, System};

#[cfg(target_os = "macos")]
use std::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ProcessSnapshot {
    pub pid: u32,
    pub start_time: u64,
    pub parent_pid: Option<u32>,
    pub name: String,
    pub cpu_percent: f32,
    pub memory_bytes: u64,
    pub executable: Option<String>,
    pub command_line: Option<String>,
    pub parent_name: Option<String>,
    pub child_count: usize,
    pub file_activity: Vec<String>,
    pub network_activity: Vec<String>,
    pub signature: Option<String>,
    pub unavailable: Vec<String>,
}

impl ProcessSnapshot {
    pub fn identity(&self) -> (u32, u64) {
        (self.pid, self.start_time)
    }
}

pub struct ProcessTable {
    system: System,
    rows: Vec<ProcessSnapshot>,
    demo: bool,
}

impl ProcessTable {
    pub fn with_demo(demo: bool) -> Self {
        Self {
            system: System::new_all(),
            rows: Vec::new(),
            demo,
        }
    }

    pub fn refresh(&mut self) {
        if self.demo {
            self.rows = demo_processes();
            return;
        }
        self.system.refresh_processes(ProcessesToUpdate::All, true);
        let processes = self.system.processes();
        #[cfg(target_os = "macos")]
        let macos_metrics = macos_process_metrics();
        let mut refreshed: Vec<ProcessSnapshot> = processes
            .values()
            .map(|p| {
                let pid = p.pid().as_u32();
                #[cfg(target_os = "macos")]
                let fallback = macos_metrics.get(&pid);
                let parent_pid = {
                    let parent_pid = p.parent().map(Pid::as_u32);
                    #[cfg(target_os = "macos")]
                    {
                        parent_pid.or_else(|| fallback.and_then(|m| m.parent_pid))
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        parent_pid
                    }
                };
                let parent_name = parent_pid
                    .and_then(|id| processes.get(&Pid::from_u32(id)))
                    .map(|x| x.name().to_string_lossy().into_owned())
                    .or_else(|| {
                        #[cfg(target_os = "macos")]
                        {
                            parent_pid.and_then(|id| macos_metrics.get(&id).map(|m| m.name.clone()))
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            None
                        }
                    });
                let child_count = processes
                    .values()
                    .filter(|x| x.parent() == Some(Pid::from_u32(pid)))
                    .count();
                let mut unavailable = Vec::new();
                if p.exe().is_none() {
                    unavailable.push("executable path".into());
                }
                if p.cwd().is_none() {
                    unavailable.push("working directory".into());
                }
                #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                unavailable.push("platform signature metadata".into());
                ProcessSnapshot {
                    pid,
                    start_time: p.start_time(),
                    parent_pid,
                    name: p.name().to_string_lossy().into_owned(),
                    cpu_percent: {
                        #[cfg(target_os = "macos")]
                        {
                            fallback.map_or_else(|| p.cpu_usage(), |m| m.cpu_percent)
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            p.cpu_usage()
                        }
                    },
                    // sysinfo 0.33 reports process memory in bytes.
                    memory_bytes: {
                        #[cfg(target_os = "macos")]
                        {
                            fallback.map_or_else(|| p.memory(), |m| m.memory_bytes)
                        }
                        #[cfg(not(target_os = "macos"))]
                        {
                            p.memory()
                        }
                    },
                    executable: p.exe().map(|x| x.to_string_lossy().into_owned()),
                    command_line: Some(
                        p.cmd()
                            .iter()
                            .map(|x| x.to_string_lossy())
                            .collect::<Vec<_>>()
                            .join(" "),
                    )
                    .filter(|x| !x.is_empty()),
                    parent_name,
                    child_count,
                    file_activity: Vec::new(),
                    network_activity: Vec::new(),
                    signature: None,
                    unavailable,
                }
            })
            .collect();

        // Keep the visible order stable while metrics change. New processes are
        // appended in PID order; existing processes retain their prior place.
        let mut by_identity: HashMap<(u32, u64), ProcessSnapshot> =
            refreshed.drain(..).map(|p| (p.identity(), p)).collect();
        let mut ordered = Vec::with_capacity(by_identity.len());
        for old in &self.rows {
            if let Some(current) = by_identity.remove(&old.identity()) {
                ordered.push(current);
            }
        }
        let mut new_processes: Vec<_> = by_identity.into_values().collect();
        new_processes.sort_by_key(|p| (p.pid, p.start_time));
        ordered.extend(new_processes);
        self.rows = ordered;
    }

    pub fn rows(&self) -> &[ProcessSnapshot] {
        &self.rows
    }

    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&ProcessSnapshot, &ProcessSnapshot) -> std::cmp::Ordering,
    {
        self.rows.sort_by(compare);
    }
}

fn demo_processes() -> Vec<ProcessSnapshot> {
    [
        (
            412,
            12.8,
            182 * 1024 * 1024,
            "whytop-demo-agent",
            Some(1),
            "launchd",
            2,
            "/usr/local/bin/whytop-demo-agent",
        ),
        (
            928,
            8.4,
            96 * 1024 * 1024,
            "WindowServer",
            Some(1),
            "launchd",
            4,
            "/System/Library/PrivateFrameworks/WindowServer.framework/WindowServer",
        ),
        (
            1440,
            5.7,
            74 * 1024 * 1024,
            "local-model-worker",
            Some(412),
            "whytop-demo-agent",
            1,
            "/opt/local/bin/local-model-worker",
        ),
        (
            2081,
            3.2,
            48 * 1024 * 1024,
            "Code Helper",
            Some(412),
            "whytop-demo-agent",
            3,
            "/Applications/Code.app/Contents/MacOS/Code Helper",
        ),
        (
            3176,
            1.1,
            31 * 1024 * 1024,
            "Terminal",
            Some(412),
            "whytop-demo-agent",
            0,
            "/System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal",
        ),
        (
            5012,
            0.6,
            22 * 1024 * 1024,
            "notificationd",
            Some(1),
            "launchd",
            0,
            "/usr/sbin/notificationd",
        ),
    ]
    .into_iter()
    .map(
        |(pid, cpu, memory, name, parent_pid, parent_name, child_count, executable)| {
            ProcessSnapshot {
                pid,
                start_time: 1_720_000_000 + pid as u64,
                parent_pid,
                name: name.into(),
                cpu_percent: cpu,
                memory_bytes: memory,
                executable: Some(executable.into()),
                command_line: Some(name.into()),
                parent_name: Some(parent_name.into()),
                child_count,
                file_activity: Vec::new(),
                network_activity: Vec::new(),
                signature: None,
                unavailable: vec!["file activity", "network activity", "signature metadata"]
                    .into_iter()
                    .map(String::from)
                    .collect(),
            }
        },
    )
    .collect()
}

#[cfg(target_os = "macos")]
#[derive(Clone, Debug)]
struct MacProcessMetrics {
    parent_pid: Option<u32>,
    cpu_percent: f32,
    memory_bytes: u64,
    name: String,
}

#[cfg(target_os = "macos")]
fn macos_process_metrics() -> HashMap<u32, MacProcessMetrics> {
    let output = match Command::new("/bin/ps")
        .args([
            "-A", "-o", "pid=", "-o", "ppid=", "-o", "pcpu=", "-o", "rss=", "-o", "comm=",
        ])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return HashMap::new(),
    };

    parse_macos_process_metrics(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(target_os = "macos")]
fn parse_macos_process_metrics(output: &str) -> HashMap<u32, MacProcessMetrics> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let pid = fields.next()?.parse().ok()?;
            let parent_pid = fields.next()?.parse().ok().filter(|pid| *pid != 0);
            let cpu_percent = fields.next()?.parse().ok()?;
            let rss_kib: u64 = fields.next()?.parse().ok()?;
            let name = fields.collect::<Vec<_>>().join(" ");
            if name.is_empty() {
                return None;
            }
            Some((
                pid,
                MacProcessMetrics {
                    parent_pid,
                    cpu_percent,
                    memory_bytes: rss_kib.saturating_mul(1024),
                    name,
                },
            ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    fn parses_mac_process_metrics_with_padded_columns() {
        let metrics = parse_macos_process_metrics(
            "    1     0   0.0  28416 /sbin/launchd\n  512     1   0.4  69440 /usr/libexec/logd",
        );
        assert_eq!(metrics.get(&1).and_then(|m| m.parent_pid), None);
        assert_eq!(metrics.get(&512).and_then(|m| m.parent_pid), Some(1));
        assert_eq!(
            metrics.get(&512).map(|m| m.memory_bytes),
            Some(69440 * 1024)
        );
    }

    #[test]
    fn identity_includes_start_time() {
        let a = ProcessSnapshot {
            pid: 7,
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
        assert_ne!(a.identity(), (7, 2));
    }
}
