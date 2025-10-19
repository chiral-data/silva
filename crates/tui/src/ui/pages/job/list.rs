use ratatui::prelude::*;
use ratatui::widgets::*;
use crossterm::event;

use crate::data_model;
use crate::ui;
use crate::ui::layout::info::MessageLevel;

#[derive(Default, PartialEq, Clone)]
pub enum Tab {
    #[default]
    New,
    Filter,
    Search,
}

#[derive(Default, PartialEq, Clone)]
pub enum StatusFilter {
    #[default]
    All,
    Running,
    Queued,
    Completed,
    Failed,
    Created,
    Cancelled,
}

#[derive(Default, PartialEq, Clone)]
pub enum SortBy {
    #[default]
    CreatedAt,
    Status,
    Duration,
    ProjectName,
}

#[derive(Default)]
pub struct States {
    pub tab_action: Tab,
    pub job_list: ratatui::widgets::ListState,
    pub status_filter: StatusFilter,
    pub sort_by: SortBy,
    pub sort_ascending: bool,
    pub search_query: String,
    pub show_queue_info: bool,
}

pub fn render(f: &mut Frame, area: Rect, states: &mut ui::states::States, store: &data_model::Store) {
    let current_style = states.get_style(true);
    
    // Extract needed values from states to avoid borrowing issues
    let action_selected = match states.job_states.list.tab_action {
        Tab::New => 0,
        Tab::Filter => 1,
        Tab::Search => 2,
    };
    
    let status_filter = states.job_states.list.status_filter.clone();
    let sort_by = states.job_states.list.sort_by.clone();
    let sort_ascending = states.job_states.list.sort_ascending;
    let search_query = states.job_states.list.search_query.clone();
    let show_queue_info = states.job_states.list.show_queue_info;

    let job_mgr = store.job_mgr.lock().unwrap();
    let mut jobs: Vec<_> = job_mgr.jobs.values().collect();
    
    // Apply status filter
    jobs = apply_status_filter(jobs, &status_filter);
    
    // Apply search filter
    if !search_query.is_empty() {
        jobs = apply_search_filter(jobs, &search_query);
    }
    
    // Apply sorting
    apply_sorting(&mut jobs, &sort_by, sort_ascending);
    
    // Get queue information
    let (queued_count, running_count, available_slots) = job_mgr.get_queue_status();
    let queue_info = format!("Queue: {} | Running: {} | Available: {}", 
                           queued_count, running_count, available_slots);
    
    // Create enhanced job display
    let mut jobs_string: Vec<String> = vec![
        format!("{:5} {:10} {:12} {:8} {:5} {}", "ID", "Status", "Config", "Duration", "Queue", "Project"),
        "-".repeat(70),
    ];
    
    // Add queue info if enabled
    if show_queue_info {
        jobs_string.push(queue_info);
        jobs_string.push("-".repeat(70));
    }
    
    // Add filtered status info
    let filter_info = format!("Filter: {} | Sort: {} {} | Found: {}", 
                             format_status_filter(&status_filter),
                             format_sort_by(&sort_by),
                             if sort_ascending { "↑" } else { "↓" },
                             jobs.len());
    jobs_string.push(filter_info);
    jobs_string.push("-".repeat(70));
    
    // Add job entries with enhanced formatting
    for (index, job) in jobs.iter().enumerate() {
        let queue_pos = if job.status == data_model::job::JobStatus::Queued {
            format!("{:3}", index + 1)
        } else {
            "--".to_string()
        };
        
        let job_str = format_job_with_colors(job, &queue_pos);
        jobs_string.push(job_str);
    }
    
    let job_list = List::new(jobs_string)
        .block(Block::bordered().title(" Enhanced Jobs "))
        .direction(ListDirection::TopToBottom)
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("> ");
    
    // Adjust list state selection to account for header
    let mut adjusted_list_state = states.job_states.list.job_list.clone();
    let header_offset = if show_queue_info { 6 } else { 4 };
    if let Some(selected) = adjusted_list_state.selected() {
        if selected < header_offset {
            adjusted_list_state.select(Some(header_offset));
        }
    }

    let top_mid_bottom = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Max(5), Constraint::Min(1)]) 
        .split(area);
    let (top, mid, bottom) = (top_mid_bottom[0], top_mid_bottom[1], top_mid_bottom[2]);

    render_enhanced_action_bar(f, top, current_style, action_selected, &states.job_states.list);
    render_enhanced_helper_hint(f, mid, current_style, &states.job_states.list);
    f.render_stateful_widget(job_list, bottom, &mut adjusted_list_state);
}

fn apply_status_filter<'a>(jobs: Vec<&'a data_model::job::Job>, filter: &StatusFilter) -> Vec<&'a data_model::job::Job> {
    match filter {
        StatusFilter::All => jobs,
        StatusFilter::Running => jobs.into_iter().filter(|j| j.status == data_model::job::JobStatus::Running).collect(),
        StatusFilter::Queued => jobs.into_iter().filter(|j| j.status == data_model::job::JobStatus::Queued).collect(),
        StatusFilter::Completed => jobs.into_iter().filter(|j| j.status == data_model::job::JobStatus::Completed).collect(),
        StatusFilter::Failed => jobs.into_iter().filter(|j| j.status == data_model::job::JobStatus::Failed).collect(),
        StatusFilter::Created => jobs.into_iter().filter(|j| j.status == data_model::job::JobStatus::Created).collect(),
        StatusFilter::Cancelled => jobs.into_iter().filter(|j| j.status == data_model::job::JobStatus::Cancelled).collect(),
    }
}

fn apply_search_filter<'a>(jobs: Vec<&'a data_model::job::Job>, query: &str) -> Vec<&'a data_model::job::Job> {
    let query_lower = query.to_lowercase();
    jobs.into_iter()
        .filter(|job| {
            let project_name = job.project_path.as_deref()
                .and_then(|p| std::path::Path::new(p).file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            project_name.contains(&query_lower) || 
            job.id.to_string().contains(&query_lower) ||
            job.status.to_string().to_lowercase().contains(&query_lower)
        })
        .collect()
}

fn apply_sorting(jobs: &mut Vec<&data_model::job::Job>, sort_by: &SortBy, ascending: bool) {
    match sort_by {
        SortBy::CreatedAt => {
            if ascending {
                jobs.sort_by(|a, b| a.created_at.cmp(&b.created_at));
            } else {
                jobs.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
        }
        SortBy::Status => {
            if ascending {
                jobs.sort_by(|a, b| a.status.to_string().cmp(&b.status.to_string()));
            } else {
                jobs.sort_by(|a, b| b.status.to_string().cmp(&a.status.to_string()));
            }
        }
        SortBy::Duration => {
            if ascending {
                jobs.sort_by(|a, b| a.duration.cmp(&b.duration));
            } else {
                jobs.sort_by(|a, b| b.duration.cmp(&a.duration));
            }
        }
        SortBy::ProjectName => {
            if ascending {
                jobs.sort_by(|a, b| {
                    let a_name = a.project_path.as_deref()
                        .and_then(|p| std::path::Path::new(p).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    let b_name = b.project_path.as_deref()
                        .and_then(|p| std::path::Path::new(p).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    a_name.cmp(b_name)
                });
            } else {
                jobs.sort_by(|a, b| {
                    let a_name = a.project_path.as_deref()
                        .and_then(|p| std::path::Path::new(p).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    let b_name = b.project_path.as_deref()
                        .and_then(|p| std::path::Path::new(p).file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    b_name.cmp(a_name)
                });
            }
        }
    }
}

fn format_status_filter(filter: &StatusFilter) -> &str {
    match filter {
        StatusFilter::All => "All",
        StatusFilter::Running => "Running",
        StatusFilter::Queued => "Queued",
        StatusFilter::Completed => "Completed",
        StatusFilter::Failed => "Failed",
        StatusFilter::Created => "Created",
        StatusFilter::Cancelled => "Cancelled",
    }
}

fn format_sort_by(sort_by: &SortBy) -> &str {
    match sort_by {
        SortBy::CreatedAt => "Created",
        SortBy::Status => "Status",
        SortBy::Duration => "Duration",
        SortBy::ProjectName => "Project",
    }
}

fn format_job_with_colors(job: &data_model::job::Job, queue_pos: &str) -> String {
    let duration_str = match &job.duration {
        Some(d) => format!("{}s", d.as_secs()),
        None => "--".to_string(),
    };
    
    let config_str = match job.config_index {
        Some(index) => format!("@job_{}.toml", index + 1),
        None => "--".to_string(),
    };
    
    let project_name = job.project_path.as_deref()
        .and_then(|p| std::path::Path::new(p).file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("--");
    
    // Add status indicators
    let status_with_indicator = match job.status {
        data_model::job::JobStatus::Running => format!("🟢 {}", job.status),
        data_model::job::JobStatus::Queued => format!("🟡 {}", job.status),
        data_model::job::JobStatus::Completed => format!("✅ {}", job.status),
        data_model::job::JobStatus::Failed => format!("❌ {}", job.status),
        data_model::job::JobStatus::Cancelled => format!("🚫 {}", job.status),
        _ => format!("⚪ {}", job.status),
    };
    
    format!("{:5} {:15} {:12} {:8} {:5} {}", 
            job.id, status_with_indicator, config_str, duration_str, queue_pos, project_name)
}

fn render_enhanced_action_bar(f: &mut Frame, area: Rect, style: Style, selected: usize, _states: &States) {
    let titles = vec![
        "New (N)",
        "Filter (F)",
        "Search (S)",
    ];
    
    let tabs = Tabs::new(titles)
        .block(Block::bordered().title(" Actions "))
        .highlight_style(style)
        .select(selected);
    
    f.render_widget(tabs, area);
}

fn render_enhanced_helper_hint(f: &mut Frame, area: Rect, style: Style, states: &States) {
    let help_text = match states.tab_action {
        Tab::New => "Enter: Create job | D: View Details | Up/Down: Navigate | F: Filter | S: Search | I: Toggle queue info",
        Tab::Filter => "R: Running | Q: Queued | C: Completed | A: All | X: Failed | D: View Details | Enter: Cycle filters",
        Tab::Search => {
            if states.search_query.is_empty() {
                "Type to search by project name, ID, or status | D: View Details | Esc: Clear | 1-4: Sort by time/status/duration/project | Space: Toggle sort order"
            } else {
                &format!("Search: '{}' | D: View Details | Backspace: Delete | Esc: Clear", states.search_query)
            }
        }
    };
    
    let help_widget = Paragraph::new(help_text)
        .block(Block::bordered().title(" Help "))
        .style(style)
        .alignment(Alignment::Left);
    
    f.render_widget(help_widget, area);
}

pub fn handle_key(key: &event::KeyEvent, states: &mut ui::states::States, store: &data_model::Store) {
    use event::KeyCode;

    match key.code {
        // Tab navigation
        KeyCode::Char('n') | KeyCode::Char('N') => {
            let states_current = &mut states.job_states.list;
            states_current.tab_action = Tab::New;
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            let states_current = &mut states.job_states.list;
            states_current.tab_action = Tab::Filter;
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let states_current = &mut states.job_states.list;
            states_current.tab_action = Tab::Search;
        }
        
        // Filter controls
        KeyCode::Char('r') | KeyCode::Char('R') => {
            let states_current = &mut states.job_states.list;
            states_current.status_filter = StatusFilter::Running;
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            let states_current = &mut states.job_states.list;
            states_current.status_filter = StatusFilter::Queued;
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            let states_current = &mut states.job_states.list;
            states_current.status_filter = StatusFilter::Completed;
        }
        KeyCode::Char('a') | KeyCode::Char('A') => {
            let states_current = &mut states.job_states.list;
            states_current.status_filter = StatusFilter::All;
        }
        KeyCode::Char('x') | KeyCode::Char('X') => {
            let states_current = &mut states.job_states.list;
            states_current.status_filter = StatusFilter::Failed;
        }
        
        // Sort controls
        KeyCode::Char('1') => {
            let states_current = &mut states.job_states.list;
            states_current.sort_by = SortBy::CreatedAt;
        }
        KeyCode::Char('2') => {
            let states_current = &mut states.job_states.list;
            states_current.sort_by = SortBy::Status;
        }
        KeyCode::Char('3') => {
            let states_current = &mut states.job_states.list;
            states_current.sort_by = SortBy::Duration;
        }
        KeyCode::Char('4') => {
            let states_current = &mut states.job_states.list;
            states_current.sort_by = SortBy::ProjectName;
        }
        
        // Toggle sort order
        KeyCode::Char(' ') => {
            let states_current = &mut states.job_states.list;
            states_current.sort_ascending = !states_current.sort_ascending;
        }
        
        // Toggle queue info
        KeyCode::Char('i') | KeyCode::Char('I') => {
            let states_current = &mut states.job_states.list;
            states_current.show_queue_info = !states_current.show_queue_info;
        }
        
        // Search functionality
        KeyCode::Char(c) if states.job_states.list.tab_action == Tab::Search => {
            let states_current = &mut states.job_states.list;
            states_current.search_query.push(c);
        }
        KeyCode::Backspace if states.job_states.list.tab_action == Tab::Search => {
            let states_current = &mut states.job_states.list;
            states_current.search_query.pop();
        }
        KeyCode::Esc => {
            let states_current = &mut states.job_states.list;
            states_current.search_query.clear();
        }
        
        // Navigation
        KeyCode::Up => {
            let states_current = &mut states.job_states.list;
            let job_mgr = store.job_mgr.lock().unwrap();
            
            // Apply filters to get the actual displayed jobs
            let mut jobs: Vec<_> = job_mgr.jobs.values().collect();
            jobs = apply_status_filter(jobs, &states_current.status_filter);
            if !states_current.search_query.is_empty() {
                jobs = apply_search_filter(jobs, &states_current.search_query);
            }
            
            if !jobs.is_empty() {
                let header_offset = if states_current.show_queue_info { 6 } else { 4 };
                let i = match states_current.job_list.selected() {
                    Some(i) => {
                        if i <= header_offset {
                            jobs.len() + header_offset - 1
                        } else {
                            i - 1
                        }
                    }
                    None => header_offset,
                };
                states_current.job_list.select(Some(i));
            }
        }
        KeyCode::Down => {
            let states_current = &mut states.job_states.list;
            let job_mgr = store.job_mgr.lock().unwrap();
            
            // Apply filters to get the actual displayed jobs
            let mut jobs: Vec<_> = job_mgr.jobs.values().collect();
            jobs = apply_status_filter(jobs, &states_current.status_filter);
            if !states_current.search_query.is_empty() {
                jobs = apply_search_filter(jobs, &states_current.search_query);
            }
            
            if !jobs.is_empty() {
                let header_offset = if states_current.show_queue_info { 6 } else { 4 };
                let i = match states_current.job_list.selected() {
                    Some(i) => {
                        if i >= jobs.len() + header_offset - 1 {
                            header_offset
                        } else {
                            i + 1
                        }
                    }
                    None => header_offset,
                };
                states_current.job_list.select(Some(i));
            }
        }
        
        // View job details
        KeyCode::Char('d') | KeyCode::Char('D') => {
            let selected_idx = states.job_states.list.job_list.selected();
            
            if let Some(selected_idx) = selected_idx {
                let states_current = &states.job_states.list;
                let header_offset = if states_current.show_queue_info { 6 } else { 4 };
                
                if selected_idx >= header_offset {
                    let job_mgr = store.job_mgr.lock().unwrap();
                    let mut jobs: Vec<_> = job_mgr.jobs.values().collect();
                    
                    // Apply same filters and sorting as in display
                    jobs = apply_status_filter(jobs, &states_current.status_filter);
                    if !states_current.search_query.is_empty() {
                        jobs = apply_search_filter(jobs, &states_current.search_query);
                    }
                    
                    let mut jobs_mut = jobs;
                    apply_sorting(&mut jobs_mut, &states_current.sort_by, states_current.sort_ascending);
                    
                    let job_index = selected_idx - header_offset;
                    if let Some(job) = jobs_mut.get(job_index) {
                        states.job_states.set_selected_job_id(Some(job.id));
                        states.job_states.show_page = super::ShowPage::Detail;
                    }
                }
            }
        }
        
        // Enter handling
        KeyCode::Enter => {
            let tab_action = states.job_states.list.tab_action.clone();
            let selected_idx = states.job_states.list.job_list.selected();
            
            match tab_action {
                Tab::New => if store.project_sel.is_none() {
                    states.info_states.message = ("no project selected".to_string(), MessageLevel::Warn);
                } else {
                    // Check if project has multiple job configurations
                    if let Some((proj, _)) = store.project_sel.as_ref() {
                        match data_model::job::Job::get_settings_vec(proj.get_dir()) {
                            Ok(settings_vec) => {
                                if settings_vec.len() == 1 {
                                    // Single config - create job directly
                                    let project_path = proj.get_dir().display().to_string();
                                    let mut job_mgr = store.job_mgr.lock().unwrap();
                                    let new_job_id = job_mgr.create_job(Some(project_path), Some(0));
                                    drop(job_mgr);
                                    
                                    states.job_states.set_selected_job_id(Some(new_job_id));
                                    states.job_states.show_page = super::ShowPage::Detail;
                                    return;
                                } else {
                                    // Multiple configs - go to config selection
                                    states.job_states.show_page = super::ShowPage::ConfigSelect;
                                    return;
                                }
                            }
                            Err(e) => {
                                states.info_states.message = (format!("Error reading job configurations: {}", e), MessageLevel::Error);
                                return;
                            }
                        }
                    }
                }
                Tab::Filter => {
                    // Cycle through filters
                    let states_current = &mut states.job_states.list;
                    states_current.status_filter = match states_current.status_filter {
                        StatusFilter::All => StatusFilter::Running,
                        StatusFilter::Running => StatusFilter::Queued,
                        StatusFilter::Queued => StatusFilter::Completed,
                        StatusFilter::Completed => StatusFilter::Failed,
                        StatusFilter::Failed => StatusFilter::Created,
                        StatusFilter::Created => StatusFilter::Cancelled,
                        StatusFilter::Cancelled => StatusFilter::All,
                    };
                }
                Tab::Search => {
                    // Search is handled by typing
                }
            }
            
            // If we have a selected job, set it as current and go to detail
            if let Some(selected_idx) = selected_idx {
                let states_current = &states.job_states.list;
                let header_offset = if states_current.show_queue_info { 6 } else { 4 };
                
                if selected_idx >= header_offset {
                    let job_mgr = store.job_mgr.lock().unwrap();
                    let mut jobs: Vec<_> = job_mgr.jobs.values().collect();
                    
                    // Apply same filters and sorting as in display
                    jobs = apply_status_filter(jobs, &states_current.status_filter);
                    if !states_current.search_query.is_empty() {
                        jobs = apply_search_filter(jobs, &states_current.search_query);
                    }
                    
                    let mut jobs_mut = jobs;
                    apply_sorting(&mut jobs_mut, &states_current.sort_by, states_current.sort_ascending);
                    
                    let job_index = selected_idx - header_offset;
                    if let Some(job) = jobs_mut.get(job_index) {
                        states.job_states.set_selected_job_id(Some(job.id));
                        states.job_states.show_page = super::ShowPage::Detail;
                    }
                }
            }
        }
        _ => ()
    }
}
