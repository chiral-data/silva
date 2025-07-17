use std::time::SystemTime;
use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

pub const HELPER: &[&str] = &[
    "Real-time job status dashboard",
    "Up/Down: Navigate | L: Filter logs | C: Clear logs | K: Kill job | Q: Queue job | R: Refresh",
];

#[derive(Default)]
pub struct States {
    pub log_filter: String,
    pub show_details: bool,
    pub show_timeline: bool,
    pub show_resources: bool,
    pub selected_section: usize,
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let job_id = states.job_states.get_current_job_id();
    if job_id == 0 {
        let no_job = Paragraph::new("No job selected")
            .block(Block::bordered().title(" Status "))
            .alignment(Alignment::Center);
        f.render_widget(no_job, area);
        return;
    }

    let job_mgr = store.job_mgr.lock().unwrap();
    let job = match job_mgr.jobs.get(&job_id) {
        Some(job) => job,
        None => {
            let no_job = Paragraph::new("Job not found")
                .block(Block::bordered().title(" Status "))
                .alignment(Alignment::Center);
            f.render_widget(no_job, area);
            return;
        }
    };

    // Create layout for different sections
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8),  // Job overview
            Constraint::Length(6),  // Queue status
            Constraint::Min(8),     // Logs
        ])
        .split(area);

    // Render job overview
    render_job_overview(f, main_layout[0], job, &job_mgr, states);
    
    // Render queue status
    render_queue_status(f, main_layout[1], &job_mgr, job_id);
    
    // Render logs section
    render_logs_section(f, main_layout[2], job_id, &job_mgr, states);
}

fn render_job_overview(f: &mut Frame, area: Rect, job: &data_model::job::Job, _job_mgr: &data_model::job::Manager, _states: &mut ui::states::States) {
    let status_color = match job.status {
        data_model::job::JobStatus::Running => Color::Green,
        data_model::job::JobStatus::Queued => Color::Yellow,
        data_model::job::JobStatus::Completed => Color::Blue,
        data_model::job::JobStatus::Failed => Color::Red,
        data_model::job::JobStatus::Cancelled => Color::Magenta,
        _ => Color::Gray,
    };

    let status_indicator = match job.status {
        data_model::job::JobStatus::Running => "🟢",
        data_model::job::JobStatus::Queued => "🟡",
        data_model::job::JobStatus::Completed => "✅",
        data_model::job::JobStatus::Failed => "❌",
        data_model::job::JobStatus::Cancelled => "🚫",
        _ => "⚪",
    };

    // Create job info text
    let created_at = format_time(job.created_at);
    let started_at = job.started_at.map(format_time).unwrap_or("--".to_string());
    let completed_at = job.completed_at.map(format_time).unwrap_or("--".to_string());
    let duration = job.duration.map(|d| format!("{}s", d.as_secs())).unwrap_or("--".to_string());
    
    let project_name = job.project_path.as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let config_name = match job.config_index {
        Some(index) => format!("@job_{}.toml", index + 1),
        None => "Unknown".to_string(),
    };

    let exit_code = job.exit_code.map(|c| c.to_string()).unwrap_or("--".to_string());

    let job_info = vec![
        format!("Job ID: {}  {}", job.id, status_indicator),
        format!("Status: {}", job.status),
        format!("Project: {} | Config: {}", project_name, config_name),
        format!("Created: {} | Started: {} | Completed: {}", created_at, started_at, completed_at),
        format!("Duration: {} | Exit Code: {}", duration, exit_code),
    ];

    let overview_widget = Paragraph::new(job_info.join("\n"))
        .block(Block::bordered().title(" Job Overview "))
        .style(Style::default().fg(status_color))
        .alignment(Alignment::Left);

    f.render_widget(overview_widget, area);
}

fn render_queue_status(f: &mut Frame, area: Rect, job_mgr: &data_model::job::Manager, job_id: usize) {
    let (queued_count, running_count, available_slots) = job_mgr.get_queue_status();
    let max_concurrent = job_mgr.max_concurrent_jobs;
    
    // Check if current job is in queue
    let job_queue_pos = job_mgr.job_queue.iter().position(|&id| id == job_id)
        .map(|pos| (pos + 1).to_string())
        .unwrap_or("--".to_string());

    let queue_info = vec![
        format!("Queue Status: {} queued, {} running, {} available", queued_count, running_count, available_slots),
        format!("Max Concurrent Jobs: {}", max_concurrent),
        format!("This Job Queue Position: {}", job_queue_pos),
    ];

    let queue_widget = Paragraph::new(queue_info.join("\n"))
        .block(Block::bordered().title(" Queue Information "))
        .alignment(Alignment::Left);

    f.render_widget(queue_widget, area);
}

fn render_logs_section(f: &mut Frame, area: Rect, job_id: usize, job_mgr: &data_model::job::Manager, states: &mut ui::states::States) {
    let mut logs: Vec<String> = job_mgr.logs.get(&job_id)
        .map(|v| v.iter().cloned().collect())
        .unwrap_or_default();

    // Add temporary log if available
    if let Some(log_tmp) = job_mgr.logs_tmp.get(&job_id) {
        logs.push(format!("🔄 {}", log_tmp));
    }

    // Apply log filter if set
    let log_filter = &states.job_states.detail.status.log_filter;
    if !log_filter.is_empty() {
        logs = logs.into_iter()
            .filter(|log| log.to_lowercase().contains(&log_filter.to_lowercase()))
            .collect();
    }

    // Add log count info and controls
    let log_count_info = if log_filter.is_empty() {
        format!("Logs ({} entries) - L: Filter, C: Clear", logs.len())
    } else {
        format!("Logs ({} entries, filtered by '{}') - Esc: Clear filter", logs.len(), log_filter)
    };

    let logs_text = if logs.is_empty() {
        if log_filter.is_empty() {
            "No logs available".to_string()
        } else {
            format!("No logs match filter '{}'", log_filter)
        }
    } else {
        logs.join("\n")
    };

    let logs_widget = Paragraph::new(logs_text)
        .block(Block::bordered().title(log_count_info))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
        .scroll((0, 0));

    f.render_widget(logs_widget, area);
}

fn format_time(time: SystemTime) -> String {
    match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(duration) => {
            let secs = duration.as_secs();
            match chrono::DateTime::from_timestamp(secs as i64, 0) {
                Some(dt) => dt.naive_utc().format("%Y-%m-%d %H:%M:%S").to_string(),
                None => "Invalid time".to_string(),
            }
        }
        Err(_) => "Invalid time".to_string(),
    }
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        KeyCode::Char('l') | KeyCode::Char('L') => {
            // Start log filter input mode
            // For now, we'll just clear the filter to start fresh
            states.job_states.detail.status.log_filter.clear();
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            // Clear log filter
            states.job_states.detail.status.log_filter.clear();
        }
        KeyCode::Char('k') | KeyCode::Char('K') => {
            // Kill/Cancel job
            let job_id = states.job_states.get_current_job_id();
            if job_id != 0 {
                let mut job_mgr = store.job_mgr.lock().unwrap();
                if let Some(job) = job_mgr.jobs.get(&job_id) {
                    if job.status == data_model::job::JobStatus::Running {
                        // Set cancellation flag
                        job_mgr.local_infra_cancel_job = true;
                        job_mgr.add_log(job_id, "Job cancellation requested".to_string());
                    } else if job.status == data_model::job::JobStatus::Queued {
                        // Cancel queued job
                        if let Ok(()) = job_mgr.cancel_queued_job(job_id) {
                            job_mgr.add_log(job_id, "Job cancelled (was queued)".to_string());
                        }
                    }
                }
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            // Queue job
            let job_id = states.job_states.get_current_job_id();
            if job_id != 0 {
                let mut job_mgr = store.job_mgr.lock().unwrap();
                if let Some(job) = job_mgr.jobs.get(&job_id) {
                    if job.can_run() {
                        if let Ok(()) = job_mgr.queue_job(job_id) {
                            job_mgr.add_log(job_id, "Job queued for execution".to_string());
                        }
                    }
                }
            }
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Refresh action - could trigger a refresh of job status
            // For now, just a placeholder
        }
        KeyCode::Char(c) if !states.job_states.detail.status.log_filter.is_empty() || c.is_alphanumeric() => {
            // Add to log filter
            states.job_states.detail.status.log_filter.push(c);
        }
        KeyCode::Backspace => {
            // Remove from log filter
            states.job_states.detail.status.log_filter.pop();
        }
        KeyCode::Esc => {
            // Clear log filter
            states.job_states.detail.status.log_filter.clear();
        }
        KeyCode::Enter => {
            // Toggle sections or apply filter
            let status_states = &mut states.job_states.detail.status;
            status_states.show_details = !status_states.show_details;
        }
        KeyCode::Up => {
            // Navigate sections
            let status_states = &mut states.job_states.detail.status;
            if status_states.selected_section > 0 {
                status_states.selected_section -= 1;
            }
        }
        KeyCode::Down => {
            // Navigate sections
            let status_states = &mut states.job_states.detail.status;
            if status_states.selected_section < 2 {
                status_states.selected_section += 1;
            }
        }
        _ => {}
    }
}