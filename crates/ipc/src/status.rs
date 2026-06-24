use std::collections::VecDeque;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ProcessState {
    Starting = 1,
    Running = 2,
    Stopping = 3,
    Stopped = 4,
    Failed = 5,
}

impl ProcessState {
    pub fn as_str(self) -> &'static str {
        match self {
            ProcessState::Starting => "starting",
            ProcessState::Running => "running",
            ProcessState::Stopping => "stopping",
            ProcessState::Stopped => "stopped",
            ProcessState::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum BrowserState {
    Creating = 1,
    Ready = 2,
    Loading = 3,
    Closing = 4,
    Closed = 5,
    Failed = 6,
}

impl BrowserState {
    pub fn as_str(self) -> &'static str {
        match self {
            BrowserState::Creating => "creating",
            BrowserState::Ready => "ready",
            BrowserState::Loading => "loading",
            BrowserState::Closing => "closing",
            BrowserState::Closed => "closed",
            BrowserState::Failed => "failed",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StatusCounters {
    pub submitted_frame_count: u64,
    pub published_frame_count: u64,
    pub dropped_paint_count: u64,
    pub dropped_output_count: u64,
}

impl StatusCounters {
    pub fn record_frame_submitted(&mut self) {
        self.submitted_frame_count = self.submitted_frame_count.saturating_add(1);
    }

    pub fn record_frame_published(&mut self) {
        self.published_frame_count = self.published_frame_count.saturating_add(1);
    }

    pub fn record_paint_dropped(&mut self) {
        self.dropped_paint_count = self.dropped_paint_count.saturating_add(1);
    }

    pub fn record_output_dropped(&mut self) {
        self.dropped_output_count = self.dropped_output_count.saturating_add(1);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatusSnapshot {
    pub process_state: ProcessState,
    pub browser_state: BrowserState,
    pub counters: StatusCounters,
    pub last_error: Option<String>,
}

impl StatusSnapshot {
    pub fn to_lines(&self) -> Vec<String> {
        let mut lines = vec![
            format!("process_state={}", self.process_state.as_str()),
            format!("browser_state={}", self.browser_state.as_str()),
            format!(
                "submitted_frame_count={}",
                self.counters.submitted_frame_count
            ),
            format!(
                "published_frame_count={}",
                self.counters.published_frame_count
            ),
            format!("dropped_paint_count={}", self.counters.dropped_paint_count),
            format!(
                "dropped_output_count={}",
                self.counters.dropped_output_count
            ),
        ];

        if let Some(last_error) = &self.last_error {
            lines.push(format!("last_error={last_error}"));
        }

        lines
    }

    pub fn encode_json_like_string(&self) -> String {
        let last_error = match &self.last_error {
            Some(last_error) => format!("\"{}\"", escape_json_like(last_error)),
            None => "null".to_string(),
        };

        format!(
            "{{\"process_state\":\"{}\",\"browser_state\":\"{}\",\"submitted_frame_count\":{},\"published_frame_count\":{},\"dropped_paint_count\":{},\"dropped_output_count\":{},\"last_error\":{}}}",
            self.process_state.as_str(),
            self.browser_state.as_str(),
            self.counters.submitted_frame_count,
            self.counters.published_frame_count,
            self.counters.dropped_paint_count,
            self.counters.dropped_output_count,
            last_error
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum OutputEventKind {
    Console = 1,
    JavaScriptResult = 2,
    PageError = 3,
    Navigation = 4,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputEvent {
    pub kind: OutputEventKind,
    pub sequence: u64,
    pub payload: String,
}

impl OutputEvent {
    pub fn new(kind: OutputEventKind, sequence: u64, payload: impl Into<String>) -> Self {
        Self {
            kind,
            sequence,
            payload: payload.into(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct OutputQueueFull {
    pub capacity: usize,
}

impl std::fmt::Display for OutputQueueFull {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "output event queue is full: capacity={}",
            self.capacity
        )
    }
}

impl std::error::Error for OutputQueueFull {}

pub type OutputQueueResult<T> = Result<T, OutputQueueFull>;

pub struct OutputQueueState {
    events: VecDeque<OutputEvent>,
    capacity: usize,
    dropped_count: u64,
}

impl OutputQueueState {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(capacity),
            capacity,
            dropped_count: 0,
        }
    }

    pub fn push_event(&mut self, event: OutputEvent) -> OutputQueueResult<()> {
        if self.events.len() >= self.capacity {
            self.dropped_count = self.dropped_count.saturating_add(1);
            return Err(OutputQueueFull {
                capacity: self.capacity,
            });
        }

        self.events.push_back(event);
        Ok(())
    }

    pub fn pop_event(&mut self) -> OutputQueueResult<Option<OutputEvent>> {
        Ok(self.events.pop_front())
    }

    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped_count
    }
}

fn escape_json_like(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            other => escaped.push(other),
        }
    }
    escaped
}
