use crate::{
    ai::{AiEngine, AiEvent, DemoEngine, OpenAiLocalEngine},
    process::ProcessTable,
};
use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap},
    Terminal,
};
use std::{
    cmp::Ordering,
    io::{self, Stdout},
    time::{Duration, Instant},
};
use tokio::{sync::mpsc, task::JoinHandle};
use tokio_util::sync::CancellationToken;

const PROCESS_REFRESH_INTERVAL: Duration = Duration::from_millis(500);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum View {
    Monitor,
    Chat,
}

struct ChatTurn {
    question: String,
    answer: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SortMode {
    Normal,
    NameAsc,
    NameDesc,
    CpuDesc,
    CpuAsc,
    MemoryDesc,
    MemoryAsc,
}

impl SortMode {
    fn next_for(self, key: char) -> Self {
        match key {
            'n' => match self {
                Self::NameAsc => Self::NameDesc,
                Self::NameDesc => Self::Normal,
                _ => Self::NameAsc,
            },
            'c' => match self {
                Self::CpuDesc => Self::CpuAsc,
                Self::CpuAsc => Self::Normal,
                _ => Self::CpuDesc,
            },
            'm' => match self {
                Self::MemoryDesc => Self::MemoryAsc,
                Self::MemoryAsc => Self::Normal,
                _ => Self::MemoryDesc,
            },
            _ => self,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::NameAsc => "name ↑",
            Self::NameDesc => "name ↓",
            Self::CpuDesc => "CPU ↓",
            Self::CpuAsc => "CPU ↑",
            Self::MemoryDesc => "memory ↓",
            Self::MemoryAsc => "memory ↑",
        }
    }
}

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let result = App::new().run(&mut terminal).await;
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    result
}

struct App {
    table: ProcessTable,
    selected: usize,
    table_state: TableState,
    question: String,
    answer: String,
    status: String,
    rx: Option<mpsc::Receiver<AiEvent>>,
    task: Option<JoinHandle<()>>,
    cancel: CancellationToken,
    engine: Engine,
    view: View,
    chat_process: Option<crate::process::ProcessSnapshot>,
    chat_history: Vec<ChatTurn>,
    active_question: Option<String>,
    chat_scroll: usize,
    sort_mode: SortMode,
}

impl App {
    fn new() -> Self {
        let demo = std::env::var("WHYTOP_DEMO").ok().as_deref() == Some("1");
        Self {
            table: ProcessTable::with_demo(demo),
            selected: 0,
            table_state: TableState::default(),
            question: String::new(),
            answer: String::new(),
            status: "AI enabled · press Enter to explain the selected process".into(),
            rx: None,
            task: None,
            cancel: CancellationToken::new(),
            engine: if demo {
                Engine::Demo(DemoEngine)
            } else {
                Engine::Local(OpenAiLocalEngine::from_env())
            },
            view: View::Monitor,
            chat_process: None,
            chat_history: Vec::new(),
            active_question: None,
            chat_scroll: 0,
            sort_mode: if demo {
                SortMode::Normal
            } else {
                SortMode::CpuDesc
            },
        }
    }
    async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        self.table.refresh();
        self.apply_sort();
        self.table_state.select(Some(self.selected));
        let mut next_refresh = Instant::now() + PROCESS_REFRESH_INTERVAL;
        loop {
            let now = Instant::now();
            if now >= next_refresh {
                self.table.refresh();
                self.apply_sort();
                // Keep the cursor at the same row while refreshes and sorting
                // change which process occupies that row.
                self.selected = self.selected.min(self.table.rows().len().saturating_sub(1));
                self.table_state.select(Some(self.selected));
                // Advance from the previous deadline so a slow draw or a
                // blocked input poll cannot permanently push refreshes out.
                next_refresh += PROCESS_REFRESH_INTERVAL;
                if next_refresh <= now {
                    next_refresh = now + PROCESS_REFRESH_INTERVAL;
                }
            }
            self.drain_ai();
            terminal.draw(|f| self.ui(f))?;
            if event::poll(Duration::from_millis(250))? {
                if let Event::Key(key) = event::read()? {
                    if self.key(key).await {
                        break;
                    }
                }
            }
        }
        Ok(())
    }
    fn drain_ai(&mut self) {
        if let Some(rx) = &mut self.rx {
            while let Ok(e) = rx.try_recv() {
                match e {
                    AiEvent::Started => {
                        self.answer.clear();
                        self.status = format!("{} connecting…", self.engine.label());
                    }
                    AiEvent::Token(t) => self.answer.push_str(&t),
                    AiEvent::Finished { ttft_ms, tokens } => {
                        if let Some(question) = self.active_question.take() {
                            self.chat_history.push(ChatTurn {
                                question,
                                answer: std::mem::take(&mut self.answer),
                            });
                        }
                        self.status =
                            format!("answer complete · TTFT {ttft_ms}ms · {tokens} tokens")
                    }
                    AiEvent::Error(e) => self.status = e,
                }
            }
        }
        if self.task.as_ref().is_some_and(JoinHandle::is_finished) {
            self.task = None;
        }
    }
    async fn key(&mut self, key: KeyEvent) -> bool {
        if self.view == View::Chat {
            return self.chat_key(key).await;
        }
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.cancel.cancel();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected + 1 < self.table.rows().len() {
                    self.selected += 1;
                    self.table_state.select(Some(self.selected));
                }
                false
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                self.table_state.select(Some(self.selected));
                false
            }
            KeyCode::Char('g') => {
                self.selected = 0;
                self.table_state.select(Some(self.selected));
                false
            }
            KeyCode::Char('G') => {
                self.selected = self.table.rows().len().saturating_sub(1);
                self.table_state.select(Some(self.selected));
                false
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let page = 10.max(self.table.rows().len() / 2);
                self.selected =
                    (self.selected + page).min(self.table.rows().len().saturating_sub(1));
                self.table_state.select(Some(self.selected));
                false
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                let page = 10.max(self.table.rows().len() / 2);
                self.selected = self.selected.saturating_sub(page);
                self.table_state.select(Some(self.selected));
                false
            }
            KeyCode::Char('a') => {
                self.open_chat();
                false
            }
            KeyCode::Char('n') | KeyCode::Char('c') | KeyCode::Char('m') => {
                self.sort_mode = self.sort_mode.next_for(match key.code {
                    KeyCode::Char(c) => c,
                    _ => unreachable!(),
                });
                self.apply_sort();
                self.selected = self.selected.min(self.table.rows().len().saturating_sub(1));
                self.table_state.select(Some(self.selected));
                false
            }
            KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
                self.question.push(c);
                false
            }
            KeyCode::Backspace => {
                self.question.pop();
                false
            }
            KeyCode::Enter => {
                self.open_chat();
                false
            }
            _ => false,
        }
    }
    async fn chat_key(&mut self, key: KeyEvent) -> bool {
        match key.code {
            KeyCode::Esc => {
                self.view = View::Monitor;
                self.chat_process = None;
                false
            }
            KeyCode::Enter => {
                self.ask();
                false
            }
            KeyCode::Up => {
                self.chat_scroll = self.chat_scroll.saturating_add(1);
                false
            }
            KeyCode::Down => {
                self.chat_scroll = self.chat_scroll.saturating_sub(1);
                false
            }
            KeyCode::Backspace => {
                self.question.pop();
                false
            }
            KeyCode::Char(c) if c.is_ascii_graphic() || c == ' ' => {
                self.question.push(c);
                false
            }
            _ => false,
        }
    }
    fn open_chat(&mut self) {
        let Some(process) = self.table.rows().get(self.selected).cloned() else {
            return;
        };
        self.view = View::Chat;
        self.chat_process = Some(process);
        self.chat_history.clear();
        self.question.clear();
        self.answer.clear();
        self.chat_scroll = 0;
        self.status = format!("{} · Esc back to processes", self.engine.label());
        self.ask();
    }

    fn apply_sort(&mut self) {
        let mode = self.sort_mode;
        self.table.sort_by(|a, b| {
            let ordering = match mode {
                SortMode::Normal => (a.pid, a.start_time).cmp(&(b.pid, b.start_time)),
                SortMode::NameAsc | SortMode::NameDesc => a
                    .name
                    .to_ascii_lowercase()
                    .cmp(&b.name.to_ascii_lowercase())
                    .then_with(|| a.name.cmp(&b.name)),
                SortMode::CpuDesc | SortMode::CpuAsc => a
                    .cpu_percent
                    .partial_cmp(&b.cpu_percent)
                    .unwrap_or(Ordering::Equal),
                SortMode::MemoryDesc | SortMode::MemoryAsc => a.memory_bytes.cmp(&b.memory_bytes),
            };
            let ordering = match mode {
                SortMode::NameDesc | SortMode::CpuAsc | SortMode::MemoryAsc => ordering.reverse(),
                _ => ordering,
            };
            ordering.then_with(|| (a.pid, a.start_time).cmp(&(b.pid, b.start_time)))
        });
    }
    fn ask(&mut self) {
        if self.task.is_some() {
            return;
        }
        let Some(snapshot) = self.chat_process.clone() else {
            return;
        };
        let question = if self.question.is_empty() {
            "Explain this process and any notable evidence.".into()
        } else {
            std::mem::take(&mut self.question)
        };
        let model_question = if self.chat_history.is_empty() {
            question.clone()
        } else {
            let history = self
                .chat_history
                .iter()
                .map(|turn| format!("USER: {}\nASSISTANT: {}", turn.question, turn.answer))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!("Previous conversation:\n{history}\n\nUSER: {question}")
        };
        self.active_question = Some(question);
        let (tx, rx) = mpsc::channel(32);
        self.rx = Some(rx);
        self.cancel = CancellationToken::new();
        let cancel = self.cancel.clone();
        let engine = self.engine.clone();
        self.task = Some(tokio::spawn(async move {
            if let Err(e) = engine
                .explain(snapshot, model_question, tx.clone(), cancel)
                .await
            {
                let _ = tx
                    .send(AiEvent::Error(format!("AI unavailable: {e}")))
                    .await;
            }
        }));
    }
    fn ui(&mut self, f: &mut ratatui::Frame) {
        if self.view == View::Chat {
            self.chat_ui(f);
            return;
        }
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(5),
            ])
            .split(f.area());
        let title = Paragraph::new(Line::from(vec![
            Span::styled(
                "whytop",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "  read-only process monitor  ·  n name  c CPU  m memory  ·  sort: {}  ·  ↑/↓ select  ·  Enter chat  ·  q quit",
                self.sort_mode.label()
            )),
        ]))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, chunks[0]);
        let rows = self.table.rows().iter().map(|p| {
            Row::new(vec![
                Cell::from(p.pid.to_string()),
                Cell::from(p.name.clone()),
                Cell::from(format!("{:.1}%", p.cpu_percent)),
                Cell::from(format_bytes(p.memory_bytes)),
                Cell::from(p.parent_name.clone().unwrap_or_else(|| "—".into())),
            ])
        });
        let table = Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Min(20),
                Constraint::Length(10),
                Constraint::Length(12),
                Constraint::Min(20),
            ],
        )
        .row_highlight_style(
            Style::default()
                .fg(Color::White)
                .bg(Color::Blue)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ")
        .header(
            Row::new(vec!["PID", "NAME", "CPU", "MEMORY", "PARENT"])
                .style(Style::default().fg(Color::Yellow)),
        )
        .block(Block::default().title("processes").borders(Borders::ALL));
        f.render_stateful_widget(table, chunks[1], &mut self.table_state);
        let selected = self.table.rows().get(self.selected);
        let detail = selected
            .map(|p| {
                format!(
                    "pid={}  start={}  exe={:?}\nchildren={}  unavailable={:?}",
                    p.pid, p.start_time, p.executable, p.child_count, p.unavailable
                )
            })
            .unwrap_or_else(|| "No processes visible".into());
        f.render_widget(
            Paragraph::new(detail)
                .wrap(Wrap { trim: false })
                .block(Block::default().title("details").borders(Borders::ALL)),
            chunks[2],
        );
    }

    fn chat_ui(&self, f: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(4),
            ])
            .split(f.area());
        let process_name = self
            .chat_process
            .as_ref()
            .map(|p| format!("{} (PID {})", p.name, p.pid))
            .unwrap_or_else(|| "selected process".into());
        let title = Paragraph::new(format!(
            "Chat with {}\n{}",
            process_name, "Enter send · ↑/↓ scroll · Esc back"
        ))
        .block(Block::default().title("process chat").borders(Borders::ALL));
        f.render_widget(title, chunks[0]);

        let mut transcript = String::new();
        for turn in &self.chat_history {
            transcript.push_str("> ");
            transcript.push_str(&turn.question);
            transcript.push_str("\n");
            transcript.push_str(&turn.answer);
            transcript.push_str("\n\n");
        }
        if let Some(question) = &self.active_question {
            transcript.push_str("> ");
            transcript.push_str(question);
            transcript.push_str("\n");
            transcript.push_str(&self.answer);
        } else if !self.answer.is_empty() {
            transcript.push_str(&self.answer);
        }
        if transcript.is_empty() {
            transcript.push_str(&self.status);
        }
        let viewport_width = chunks[1].width.saturating_sub(2) as usize;
        let viewport_height = chunks[1].height.saturating_sub(2);
        let transcript_lines = wrapped_line_count(&transcript, viewport_width);
        let max_scroll = transcript_lines.saturating_sub(viewport_height as usize);
        let scroll = max_scroll
            .saturating_sub(self.chat_scroll)
            .min(u16::MAX as usize) as u16;
        f.render_widget(
            Paragraph::new(transcript)
                .wrap(Wrap { trim: false })
                .scroll((scroll, 0))
                .block(Block::default().title("conversation").borders(Borders::ALL)),
            chunks[1],
        );

        let input = if self.question.is_empty() {
            "Type a follow-up question and press Enter".into()
        } else {
            format!("> {}", self.question)
        };
        f.render_widget(
            Paragraph::new(input).wrap(Wrap { trim: false }).block(
                Block::default()
                    .title(self.status.as_str())
                    .borders(Borders::ALL),
            ),
            chunks[2],
        );
    }
}

#[derive(Clone)]
enum Engine {
    Local(OpenAiLocalEngine),
    Demo(DemoEngine),
}

#[async_trait::async_trait]
impl AiEngine for Engine {
    async fn explain(
        &self,
        snapshot: crate::process::ProcessSnapshot,
        question: String,
        events: mpsc::Sender<AiEvent>,
        cancel: CancellationToken,
    ) -> Result<()> {
        match self {
            Self::Local(engine) => engine.explain(snapshot, question, events, cancel).await,
            Self::Demo(engine) => engine.explain(snapshot, question, events, cancel).await,
        }
    }
    fn label(&self) -> &'static str {
        match self {
            Self::Local(engine) => engine.label(),
            Self::Demo(engine) => engine.label(),
        }
    }
}

fn wrapped_line_count(text: &str, width: usize) -> usize {
    let width = width.max(1);
    text.lines()
        .map(|line| {
            let characters = line.chars().count();
            characters.max(1).div_ceil(width)
        })
        .sum()
}

fn format_bytes(n: u64) -> String {
    if n >= 1024 * 1024 * 1024 {
        format!("{:.1}G", n as f64 / 1024f64.powi(3))
    } else if n >= 1024 * 1024 {
        format!("{:.1}M", n as f64 / 1024f64.powi(2))
    } else if n >= 1024 {
        format!("{:.0}K", n as f64 / 1024f64)
    } else {
        format!("{}B", n)
    }
}

#[cfg(test)]
mod tests {
    use super::SortMode::*;

    #[test]
    fn sort_keys_cycle_through_direction_and_normal() {
        assert_eq!(Normal.next_for('n'), NameAsc);
        assert_eq!(NameAsc.next_for('n'), NameDesc);
        assert_eq!(NameDesc.next_for('n'), Normal);

        assert_eq!(Normal.next_for('c'), CpuDesc);
        assert_eq!(CpuDesc.next_for('c'), CpuAsc);
        assert_eq!(CpuAsc.next_for('c'), Normal);

        assert_eq!(Normal.next_for('m'), MemoryDesc);
        assert_eq!(MemoryDesc.next_for('m'), MemoryAsc);
        assert_eq!(MemoryAsc.next_for('m'), Normal);
    }
}
