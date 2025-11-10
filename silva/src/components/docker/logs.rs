use chrono::{DateTime, Utc};
use std::{collections::VecDeque, fmt::Display};

/// Source of a log line (stdout or stderr).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSource {
    Stdout,
    Stderr,
}

/// A single line of log output.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: DateTime<Utc>,
    pub source: LogSource,
    pub content: String,
}

impl Display for LogLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let source_prefix = match self.source {
            LogSource::Stdout => "[OUT]",
            LogSource::Stderr => "[ERR]",
        };
        let log_string = format!(
            "{} {} {}",
            self.timestamp.format("%H:%M:%S"),
            source_prefix,
            self.content
        );
        write!(f, "{log_string}")
    }
}

impl LogLine {
    pub fn new(source: LogSource, content: String) -> Self {
        Self {
            timestamp: Utc::now(),
            source,
            content,
        }
    }

    pub fn empty() -> Self {
        Self::new(LogSource::Stdout, "".to_string())
    }
}

/// A circular buffer for storing log lines.
#[derive(Debug, Clone)]
pub struct LogBuffer {
    lines: VecDeque<LogLine>,
    max_size: usize,
}

impl LogBuffer {
    /// Creates a new log buffer with the specified maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            lines: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Adds a log line to the buffer.
    /// If the buffer is full, the oldest line is removed.
    pub fn push(&mut self, line: LogLine) {
        if self.lines.len() >= self.max_size {
            self.lines.pop_front();
        }
        self.lines.push_back(line);
    }

    /// Returns all log lines in the buffer.
    pub fn lines(&self) -> &VecDeque<LogLine> {
        &self.lines
    }

    /// Returns the number of lines in the buffer.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Returns true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Clears all log lines from the buffer.
    pub fn clear(&mut self) {
        self.lines.clear();
    }

    /// Returns the last N lines from the buffer.
    pub fn tail(&self, n: usize) -> Vec<&LogLine> {
        self.lines.iter().rev().take(n).rev().collect()
    }

    // Append logs from another LogBuffer
    pub fn append(&mut self, source: &mut Self) {
        self.lines.append(&mut source.lines);
    }
}

impl Default for LogBuffer {
    fn default() -> Self {
        Self::new(10000)
    }
}

impl Display for LogBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let log_string = self
            .lines
            .iter()
            .map(|line| {
                let source_prefix = match line.source {
                    LogSource::Stdout => "[OUT]",
                    LogSource::Stderr => "[ERR]",
                };
                format!(
                    "{} {} {}",
                    line.timestamp.format("%H:%M:%S"),
                    source_prefix,
                    line.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        write!(f, "{log_string}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_buffer_push() {
        let mut buffer = LogBuffer::new(3);
        buffer.push(LogLine::new(LogSource::Stdout, "line 1".to_string()));
        buffer.push(LogLine::new(LogSource::Stdout, "line 2".to_string()));
        buffer.push(LogLine::new(LogSource::Stdout, "line 3".to_string()));

        assert_eq!(buffer.len(), 3);
    }

    #[test]
    fn test_log_buffer_rotation() {
        let mut buffer = LogBuffer::new(2);
        buffer.push(LogLine::new(LogSource::Stdout, "line 1".to_string()));
        buffer.push(LogLine::new(LogSource::Stdout, "line 2".to_string()));
        buffer.push(LogLine::new(LogSource::Stdout, "line 3".to_string()));

        assert_eq!(buffer.len(), 2);
        assert_eq!(buffer.lines()[0].content, "line 2");
        assert_eq!(buffer.lines()[1].content, "line 3");
    }

    #[test]
    fn test_log_buffer_clear() {
        let mut buffer = LogBuffer::new(10);
        buffer.push(LogLine::new(LogSource::Stdout, "line 1".to_string()));
        buffer.push(LogLine::new(LogSource::Stdout, "line 2".to_string()));

        assert_eq!(buffer.len(), 2);

        buffer.clear();
        assert_eq!(buffer.len(), 0);
        assert!(buffer.is_empty());
    }

    #[test]
    fn test_log_buffer_tail() {
        let mut buffer = LogBuffer::new(10);
        buffer.push(LogLine::new(LogSource::Stdout, "line 1".to_string()));
        buffer.push(LogLine::new(LogSource::Stdout, "line 2".to_string()));
        buffer.push(LogLine::new(LogSource::Stdout, "line 3".to_string()));

        let tail = buffer.tail(2);
        assert_eq!(tail.len(), 2);
        assert_eq!(tail[0].content, "line 2");
        assert_eq!(tail[1].content, "line 3");
    }

    #[test]
    fn test_log_line_source() {
        let stdout_line = LogLine::new(LogSource::Stdout, "stdout message".to_string());
        let stderr_line = LogLine::new(LogSource::Stderr, "stderr message".to_string());

        assert_eq!(stdout_line.source, LogSource::Stdout);
        assert_eq!(stderr_line.source, LogSource::Stderr);
    }
}
