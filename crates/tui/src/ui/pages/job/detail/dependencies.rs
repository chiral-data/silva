use std::time::SystemTime;
use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;

pub const HELPER: &[&str] = &[
    "Job Dependencies and Scheduling",
    "Up/Down: Navigate | Enter: Toggle dependency | A: Add dependency | R: Remove dependency",
    "S: Set schedule | P: Set priority | C: Clear schedule | D: Delete dependency",
];

#[derive(Default)]
pub struct States {
    pub selected_job: Option<usize>,
    pub selected_dependency: usize,
    pub show_schedule: bool,
    pub show_priority: bool,
    pub input_mode: InputMode,
    pub input_buffer: String,
}

#[derive(Default, PartialEq, Clone)]
enum InputMode {
    #[default]
    Normal,
    AddDependency,
    SetSchedule,
    SetPriority,
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let job_id = states.job_states.get_current_job_id();
    if job_id == 0 {
        let no_job = Paragraph::new("No job selected")
            .block(Block::bordered().title(" Dependencies "))
            .alignment(Alignment::Center);
        f.render_widget(no_job, area);
        return;
    }

    let job_mgr = store.job_mgr.lock().unwrap();
    let job = match job_mgr.jobs.get(&job_id) {
        Some(job) => job,
        None => {
            let no_job = Paragraph::new("Job not found")
                .block(Block::bordered().title(" Dependencies "))
                .alignment(Alignment::Center);
            f.render_widget(no_job, area);
            return;
        }
    };

    // Create layout for different sections
    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),  // Job info
            Constraint::Length(8),  // Dependencies
            Constraint::Length(6),  // Schedule & Priority
            Constraint::Min(4),     // Available jobs
        ])
        .split(area);

    // Render job info
    render_job_info(f, main_layout[0], job);
    
    // Render dependencies
    render_dependencies(f, main_layout[1], job, &job_mgr, states);
    
    // Render schedule and priority
    render_schedule_priority(f, main_layout[2], job, states);
    
    // Render available jobs
    render_available_jobs(f, main_layout[3], job_id, &job_mgr, states);
}

fn render_job_info(f: &mut Frame, area: Rect, job: &data_model::job::Job) {
    let project_name = job.project_path.as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown");

    let info_text = vec![
        format!("Job ID: {}", job.id),
        format!("Project: {}", project_name),
        format!("Status: {}", job.status),
        format!("Priority: {}", job.priority),
    ];

    let info_widget = Paragraph::new(info_text.join("\n"))
        .block(Block::bordered().title(" Job Information "))
        .alignment(Alignment::Left);

    f.render_widget(info_widget, area);
}

fn render_dependencies(f: &mut Frame, area: Rect, job: &data_model::job::Job, job_mgr: &data_model::job::Manager, states: &mut ui::states::States) {
    let mut deps_info = vec![];
    
    if job.dependencies.is_empty() {
        deps_info.push("No dependencies".to_string());
    } else {
        for (i, dep_id) in job.dependencies.iter().enumerate() {
            let dep_status = if let Some(dep_job) = job_mgr.jobs.get(dep_id) {
                let status_indicator = match dep_job.status {
                    data_model::job::JobStatus::Completed => "✅",
                    data_model::job::JobStatus::Running => "🟢",
                    data_model::job::JobStatus::Failed => "❌",
                    data_model::job::JobStatus::Queued => "🟡",
                    _ => "⚪",
                };
                
                let project_name = dep_job.project_path.as_deref()
                    .and_then(|p| std::path::Path::new(p).file_name())
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");
                
                format!("{}. {} Job {} ({}) - {}", i + 1, status_indicator, dep_id, project_name, dep_job.status)
            } else {
                format!("{}. ❌ Job {} (Not Found)", i + 1, dep_id)
            };
            
            if i == states.job_states.detail.dependencies.selected_dependency {
                deps_info.push(format!("> {}", dep_status));
            } else {
                deps_info.push(format!("  {}", dep_status));
            }
        }
    }

    let deps_widget = Paragraph::new(deps_info.join("\n"))
        .block(Block::bordered().title(" Dependencies "))
        .alignment(Alignment::Left);

    f.render_widget(deps_widget, area);
}

fn render_schedule_priority(f: &mut Frame, area: Rect, job: &data_model::job::Job, states: &mut ui::states::States) {
    let mut schedule_info = vec![];
    
    // Priority info
    schedule_info.push(format!("Priority: {} (higher = more important)", job.priority));
    
    // Schedule info
    if let Some(scheduled_at) = job.scheduled_at {
        let scheduled_str = format_time(scheduled_at);
        let now = SystemTime::now();
        let status = if now >= scheduled_at {
            "Ready to run"
        } else {
            "Scheduled for future"
        };
        schedule_info.push(format!("Scheduled: {} ({})", scheduled_str, status));
    } else {
        schedule_info.push("Schedule: Run immediately".to_string());
    }
    
    // Recurring info
    if let Some(recurring) = &job.recurring {
        let interval_str = match &recurring.interval {
            data_model::job::RecurringInterval::Hourly => "Every hour",
            data_model::job::RecurringInterval::Daily => "Every day",
            data_model::job::RecurringInterval::Weekly => "Every week",
            data_model::job::RecurringInterval::Monthly => "Every month",
            data_model::job::RecurringInterval::Custom(duration) => {
                &format!("Every {} seconds", duration.as_secs())
            }
        };
        schedule_info.push(format!("Recurring: {}", interval_str));
    } else {
        schedule_info.push("Recurring: No".to_string());
    }

    // Input mode info
    match states.job_states.detail.dependencies.input_mode {
        InputMode::AddDependency => {
            schedule_info.push("".to_string());
            schedule_info.push(format!("Add dependency ID: {}", states.job_states.detail.dependencies.input_buffer));
        }
        InputMode::SetSchedule => {
            schedule_info.push("".to_string());
            schedule_info.push(format!("Set schedule (seconds from now): {}", states.job_states.detail.dependencies.input_buffer));
        }
        InputMode::SetPriority => {
            schedule_info.push("".to_string());
            schedule_info.push(format!("Set priority: {}", states.job_states.detail.dependencies.input_buffer));
        }
        InputMode::Normal => {}
    }

    let schedule_widget = Paragraph::new(schedule_info.join("\n"))
        .block(Block::bordered().title(" Schedule & Priority "))
        .alignment(Alignment::Left);

    f.render_widget(schedule_widget, area);
}

fn render_available_jobs(f: &mut Frame, area: Rect, current_job_id: usize, job_mgr: &data_model::job::Manager, _states: &mut ui::states::States) {
    let mut available_jobs = vec![];
    
    // Get all jobs that could be dependencies (completed or running)
    let mut jobs: Vec<_> = job_mgr.jobs.values()
        .filter(|job| job.id != current_job_id && job.id < current_job_id) // Only allow dependencies on earlier jobs
        .collect();
    
    jobs.sort_by_key(|job| job.id);
    
    if jobs.is_empty() {
        available_jobs.push("No jobs available as dependencies".to_string());
    } else {
        available_jobs.push("Available jobs (only earlier jobs can be dependencies):".to_string());
        for job in jobs {
            let project_name = job.project_path.as_deref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("Unknown");
            
            let status_indicator = match job.status {
                data_model::job::JobStatus::Completed => "✅",
                data_model::job::JobStatus::Running => "🟢",
                data_model::job::JobStatus::Failed => "❌",
                data_model::job::JobStatus::Queued => "🟡",
                _ => "⚪",
            };
            
            available_jobs.push(format!("  {} Job {} ({}) - {}", status_indicator, job.id, project_name, job.status));
        }
    }

    let available_widget = Paragraph::new(available_jobs.join("\n"))
        .block(Block::bordered().title(" Available Jobs "))
        .alignment(Alignment::Left);

    f.render_widget(available_widget, area);
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
    
    let job_id = states.job_states.get_current_job_id();
    let input_mode = states.job_states.detail.dependencies.input_mode.clone();
    
    match input_mode {
        InputMode::Normal => {
            match key.code {
                KeyCode::Up => {
                    if job_id != 0 {
                        let job_mgr = store.job_mgr.lock().unwrap();
                        if let Some(job) = job_mgr.jobs.get(&job_id) {
                            let dep_states = &mut states.job_states.detail.dependencies;
                            if !job.dependencies.is_empty() && dep_states.selected_dependency > 0 {
                                dep_states.selected_dependency -= 1;
                            }
                        }
                    }
                }
                KeyCode::Down => {
                    if job_id != 0 {
                        let job_mgr = store.job_mgr.lock().unwrap();
                        if let Some(job) = job_mgr.jobs.get(&job_id) {
                            let dep_states = &mut states.job_states.detail.dependencies;
                            if dep_states.selected_dependency < job.dependencies.len().saturating_sub(1) {
                                dep_states.selected_dependency += 1;
                            }
                        }
                    }
                }
                KeyCode::Char('a') | KeyCode::Char('A') => {
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::AddDependency;
                    dep_states.input_buffer.clear();
                }
                KeyCode::Char('s') | KeyCode::Char('S') => {
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::SetSchedule;
                    dep_states.input_buffer.clear();
                }
                KeyCode::Char('p') | KeyCode::Char('P') => {
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::SetPriority;
                    dep_states.input_buffer.clear();
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    // Remove selected dependency
                    if job_id != 0 {
                        let selected_dep = states.job_states.detail.dependencies.selected_dependency;
                        let mut job_mgr = store.job_mgr.lock().unwrap();
                        let dep_id = if let Some(job) = job_mgr.jobs.get(&job_id) {
                            if !job.dependencies.is_empty() && selected_dep < job.dependencies.len() {
                                Some(job.dependencies[selected_dep])
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        
                        if let Some(dep_id) = dep_id {
                            if let Some(job) = job_mgr.jobs.get_mut(&job_id) {
                                job.remove_dependency(dep_id);
                                let new_len = job.dependencies.len();
                                job_mgr.add_log(job_id, format!("Removed dependency on job {}", dep_id));
                                
                                // Adjust selection
                                let dep_states = &mut states.job_states.detail.dependencies;
                                if dep_states.selected_dependency >= new_len && new_len > 0 {
                                    dep_states.selected_dependency = new_len - 1;
                                }
                            }
                        }
                    }
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    // Clear schedule
                    if job_id != 0 {
                        let mut job_mgr = store.job_mgr.lock().unwrap();
                        if let Some(job) = job_mgr.jobs.get_mut(&job_id) {
                            job.scheduled_at = None;
                            job.recurring = None;
                            job_mgr.add_log(job_id, "Cleared schedule and recurring settings".to_string());
                        }
                    }
                }
                _ => {}
            }
        }
        InputMode::AddDependency => {
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    states.job_states.detail.dependencies.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    states.job_states.detail.dependencies.input_buffer.pop();
                }
                KeyCode::Enter => {
                    if let Ok(dep_id) = states.job_states.detail.dependencies.input_buffer.parse::<usize>() {
                        if job_id != 0 && dep_id != job_id {
                            let mut job_mgr = store.job_mgr.lock().unwrap();
                            let job_exists = job_mgr.jobs.contains_key(&dep_id);
                            if let Some(job) = job_mgr.jobs.get_mut(&job_id) {
                                // Check if dependency job exists and is valid
                                if job_exists && dep_id < job_id {
                                    job.add_dependency(dep_id);
                                    job_mgr.add_log(job_id, format!("Added dependency on job {}", dep_id));
                                } else {
                                    job_mgr.add_log(job_id, format!("Cannot add dependency on job {} (not found or invalid)", dep_id));
                                }
                            }
                        }
                    }
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::Normal;
                    dep_states.input_buffer.clear();
                }
                KeyCode::Esc => {
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::Normal;
                    dep_states.input_buffer.clear();
                }
                _ => {}
            }
        }
        InputMode::SetSchedule => {
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() => {
                    states.job_states.detail.dependencies.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    states.job_states.detail.dependencies.input_buffer.pop();
                }
                KeyCode::Enter => {
                    if let Ok(seconds) = states.job_states.detail.dependencies.input_buffer.parse::<u64>() {
                        if job_id != 0 {
                            let mut job_mgr = store.job_mgr.lock().unwrap();
                            if let Some(job) = job_mgr.jobs.get_mut(&job_id) {
                                let scheduled_time = SystemTime::now() + std::time::Duration::from_secs(seconds);
                                job.set_schedule(scheduled_time);
                                job_mgr.add_log(job_id, format!("Scheduled job to run in {} seconds", seconds));
                            }
                        }
                    }
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::Normal;
                    dep_states.input_buffer.clear();
                }
                KeyCode::Esc => {
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::Normal;
                    dep_states.input_buffer.clear();
                }
                _ => {}
            }
        }
        InputMode::SetPriority => {
            match key.code {
                KeyCode::Char(c) if c.is_ascii_digit() || c == '-' => {
                    states.job_states.detail.dependencies.input_buffer.push(c);
                }
                KeyCode::Backspace => {
                    states.job_states.detail.dependencies.input_buffer.pop();
                }
                KeyCode::Enter => {
                    if let Ok(priority) = states.job_states.detail.dependencies.input_buffer.parse::<i32>() {
                        if job_id != 0 {
                            let mut job_mgr = store.job_mgr.lock().unwrap();
                            if let Some(job) = job_mgr.jobs.get_mut(&job_id) {
                                job.priority = priority;
                                job_mgr.add_log(job_id, format!("Set job priority to {}", priority));
                            }
                        }
                    }
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::Normal;
                    dep_states.input_buffer.clear();
                }
                KeyCode::Esc => {
                    let dep_states = &mut states.job_states.detail.dependencies;
                    dep_states.input_mode = InputMode::Normal;
                    dep_states.input_buffer.clear();
                }
                _ => {}
            }
        }
    }
}