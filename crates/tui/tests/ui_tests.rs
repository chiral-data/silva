use std::sync::{Arc, Mutex};
use tempfile::TempDir;
use std::fs;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use silva_tui::data_model;
use silva_tui::ui::pages::job;
use silva_tui::ui::states::States;

#[cfg(test)]
mod ui_tests {
    use super::*;

    fn create_test_store() -> data_model::Store {
        let temp_dir = TempDir::new().unwrap();
        let test_data_dir = temp_dir.path().join("silva_test");
        fs::create_dir_all(&test_data_dir).unwrap();
        
        std::env::set_var("SILVA_DATA_DIR", test_data_dir.to_str().unwrap());
        
        data_model::Store::default()
    }

    fn create_key_event(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::empty(),
            kind: crossterm::event::KeyEventKind::Press,
            state: crossterm::event::KeyEventState::empty(),
        }
    }

    fn create_test_project(temp_dir: &TempDir, name: &str, configs: Vec<&str>) -> data_model::project::Project {
        let project_dir = temp_dir.path().join(name);
        fs::create_dir_all(&project_dir).unwrap();
        
        if configs.len() == 1 {
            fs::write(project_dir.join("@job.toml"), configs[0]).unwrap();
        } else {
            for (i, config) in configs.iter().enumerate() {
                fs::write(project_dir.join(format!("@job_{}.toml", i + 1)), config).unwrap();
            }
        }
        
        fs::write(project_dir.join("README.md"), "# Test Project").unwrap();
        
        data_model::project::Project::new(project_dir)
    }

    #[test]
    fn test_job_list_navigation() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create some jobs
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job1_id = job_mgr_lock.create_job(Some("/project1".to_string()), Some(0));
        let job2_id = job_mgr_lock.create_job(Some("/project2".to_string()), Some(1));
        let job3_id = job_mgr_lock.create_job(Some("/project3".to_string()), Some(0));
        drop(job_mgr_lock);
        
        // Test navigation
        assert_eq!(states.job_states.list.job_list.selected(), None);
        
        // Simulate Down key press
        job::list::handle_key(&create_key_event(KeyCode::Down), &mut states, &store);
        assert_eq!(states.job_states.list.job_list.selected(), Some(2)); // Skip header
        
        // Simulate another Down key press
        job::list::handle_key(&create_key_event(KeyCode::Down), &mut states, &store);
        assert_eq!(states.job_states.list.job_list.selected(), Some(3));
        
        // Simulate Up key press
        job::list::handle_key(&create_key_event(KeyCode::Up), &mut states, &store);
        assert_eq!(states.job_states.list.job_list.selected(), Some(2));
    }

    #[test]
    fn test_job_list_new_job_selection() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Test with no project selected
        job::list::handle_key(&create_key_event(KeyCode::Char('n')), &mut states, &store);
        assert_eq!(states.job_states.list.tab_action, job::list::Tab::New);
        
        job::list::handle_key(&create_key_event(KeyCode::Enter), &mut states, &store);
        assert_eq!(states.info_states.message.0, "no project selected");
        
        // Test with single config project
        let single_config = r#"
[files]
inputs = ["input.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        let project = create_test_project(&temp_dir, "single_project", vec![single_config]);
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        job::list::handle_key(&create_key_event(KeyCode::Enter), &mut states, &store);
        assert_eq!(states.job_states.show_page, job::ShowPage::Detail);
        assert!(states.job_states.get_selected_job_id().is_some());
    }

    #[test]
    fn test_job_list_multiple_config_selection() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Test with multiple config project
        let config1 = r#"
[files]
inputs = ["input1.txt"]
outputs = ["output1.txt"]
scripts = ["run1.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run1.sh"
use_gpu = false
"#;
        
        let config2 = r#"
[files]
inputs = ["input2.txt"]
outputs = ["output2.txt"]
scripts = ["run2.sh"]

[local]
docker_image = "python:3.9"
script = "run2.sh"
use_gpu = true
"#;
        
        let project = create_test_project(&temp_dir, "multi_project", vec![config1, config2]);
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        states.job_states.list.tab_action = job::list::Tab::New;
        
        job::list::handle_key(&create_key_event(KeyCode::Enter), &mut states, &store);
        assert_eq!(states.job_states.show_page, job::ShowPage::ConfigSelect);
    }

    #[test]
    fn test_config_select_navigation() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create project with multiple configs
        let config1 = r#"
[files]
inputs = ["input1.txt"]
outputs = ["output1.txt"]
scripts = ["run1.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run1.sh"
use_gpu = false
"#;
        
        let config2 = r#"
[files]
inputs = ["input2.txt"]
outputs = ["output2.txt"]
scripts = ["run2.sh"]

[dok]
base_image = "python:3.9"
plan = "v100-32gb"
http_port = 8080
"#;
        
        let project = create_test_project(&temp_dir, "multi_project", vec![config1, config2]);
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        states.job_states.show_page = job::ShowPage::ConfigSelect;
        
        // Test navigation
        job::config_select::handle_key(&create_key_event(KeyCode::Down), &mut states, &store);
        assert_eq!(states.job_states.config_select.config_list.selected(), Some(0));
        
        job::config_select::handle_key(&create_key_event(KeyCode::Down), &mut states, &store);
        assert_eq!(states.job_states.config_select.config_list.selected(), Some(1));
        
        job::config_select::handle_key(&create_key_event(KeyCode::Up), &mut states, &store);
        assert_eq!(states.job_states.config_select.config_list.selected(), Some(0));
    }

    #[test]
    fn test_config_select_job_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create project with multiple configs
        let config1 = r#"
[files]
inputs = ["input1.txt"]
outputs = ["output1.txt"]
scripts = ["run1.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run1.sh"
use_gpu = false
"#;
        
        let config2 = r#"
[files]
inputs = ["input2.txt"]
outputs = ["output2.txt"]
scripts = ["run2.sh"]

[local]
docker_image = "python:3.9"
script = "run2.sh"
use_gpu = true
"#;
        
        let project = create_test_project(&temp_dir, "multi_project", vec![config1, config2]);
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        states.job_states.show_page = job::ShowPage::ConfigSelect;
        states.job_states.config_select.config_list.select(Some(1)); // Select second config
        
        // Test job creation
        job::config_select::handle_key(&create_key_event(KeyCode::Enter), &mut states, &store);
        
        assert_eq!(states.job_states.show_page, job::ShowPage::Detail);
        assert!(states.job_states.get_selected_job_id().is_some());
        
        // Verify job was created with correct config index
        let job_id = states.job_states.get_selected_job_id().unwrap();
        let job_mgr = store.job_mgr.lock().unwrap();
        let job = job_mgr.jobs.get(&job_id).unwrap();
        assert_eq!(job.config_index, Some(1));
    }

    #[test]
    fn test_config_select_escape() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        states.job_states.show_page = job::ShowPage::ConfigSelect;
        
        job::config_select::handle_key(&create_key_event(KeyCode::Esc), &mut states, &store);
        assert_eq!(states.job_states.show_page, job::ShowPage::List);
    }

    #[test]
    fn test_job_detail_tab_navigation() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create a job and select it
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job_id = job_mgr_lock.create_job(Some("/project".to_string()), Some(0));
        drop(job_mgr_lock);
        
        states.job_states.set_selected_job_id(Some(job_id));
        states.job_states.show_page = job::ShowPage::Detail;
        
        // Test tab navigation
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Files);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('p')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Pod);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('r')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Run);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('a')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Cancel);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('c')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Chat);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('f')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Files);
    }

    #[test]
    fn test_job_states_management() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Test initial state
        assert_eq!(states.job_states.get_selected_job_id(), None);
        assert_eq!(states.job_states.get_current_job_id(), 0);
        
        // Test setting job ID
        states.job_states.set_selected_job_id(Some(42));
        assert_eq!(states.job_states.get_selected_job_id(), Some(42));
        assert_eq!(states.job_states.get_current_job_id(), 42);
        
        // Test clearing job ID
        states.job_states.set_selected_job_id(None);
        assert_eq!(states.job_states.get_selected_job_id(), None);
        assert_eq!(states.job_states.get_current_job_id(), 0);
    }

    #[test]
    fn test_job_list_job_selection() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create some jobs
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job1_id = job_mgr_lock.create_job(Some("/project1".to_string()), Some(0));
        let job2_id = job_mgr_lock.create_job(Some("/project2".to_string()), Some(1));
        drop(job_mgr_lock);
        
        // Select first job (index 2 due to header)
        states.job_states.list.job_list.select(Some(2));
        
        job::list::handle_key(&create_key_event(KeyCode::Enter), &mut states, &store);
        
        assert_eq!(states.job_states.show_page, job::ShowPage::Detail);
        
        // Should select the most recent job (job2_id since jobs are sorted by creation time)
        let selected_job_id = states.job_states.get_selected_job_id().unwrap();
        assert_eq!(selected_job_id, job2_id);
    }

    #[test]
    fn test_job_info_display() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create a project and job
        let config = r#"
[files]
inputs = ["input.txt"]
outputs = ["output.txt"]
scripts = ["run.sh"]

[local]
docker_image = "ubuntu:latest"
script = "run.sh"
use_gpu = false
"#;
        
        let project = create_test_project(&temp_dir, "test_project", vec![config]);
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        let job_id = job_mgr_lock.create_job(
            Some("/path/to/test_project".to_string()), 
            Some(0)
        );
        job_mgr_lock.update_job_status(job_id, data_model::job::JobStatus::Running).unwrap();
        drop(job_mgr_lock);
        
        states.job_states.set_selected_job_id(Some(job_id));
        
        // The job detail render function should create a proper info display
        // This is tested indirectly through the job display format
        let job_mgr = store.job_mgr.lock().unwrap();
        let job = job_mgr.jobs.get(&job_id).unwrap();
        let display = format!("{}", job);
        
        assert!(display.contains(&job_id.to_string()));
        assert!(display.contains("Running"));
        assert!(display.contains("@job_1.toml"));
        assert!(display.contains("test_project"));
    }

    #[test]
    fn test_error_handling() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Test config selection with no project
        states.job_states.show_page = job::ShowPage::ConfigSelect;
        job::config_select::handle_key(&create_key_event(KeyCode::Enter), &mut states, &store);
        assert_eq!(states.info_states.message.0, "No project selected");
        
        // Test config selection with no selection
        let project = create_test_project(&temp_dir, "test_project", vec!["[files]\ninputs = []\noutputs = []\nscripts = []"]);
        let project_mgr = data_model::project::Manager::new().unwrap();
        store.project_sel = Some((project, project_mgr));
        
        job::config_select::handle_key(&create_key_event(KeyCode::Enter), &mut states, &store);
        assert_eq!(states.info_states.message.0, "No configuration selected");
    }

    #[test]
    fn test_job_display_with_different_configs() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        
        let job_mgr = store.job_mgr.clone();
        let mut job_mgr_lock = job_mgr.lock().unwrap();
        
        // Test job with no config index
        let job1_id = job_mgr_lock.create_job(Some("/project1".to_string()), None);
        let job1 = job_mgr_lock.jobs.get(&job1_id).unwrap();
        let display1 = format!("{}", job1);
        assert!(display1.contains("--")); // Should show "--" for no config
        
        // Test job with config index 0 (should show @job_1.toml)
        let job2_id = job_mgr_lock.create_job(Some("/project2".to_string()), Some(0));
        let job2 = job_mgr_lock.jobs.get(&job2_id).unwrap();
        let display2 = format!("{}", job2);
        assert!(display2.contains("@job_1.toml"));
        
        // Test job with config index 2 (should show @job_3.toml)
        let job3_id = job_mgr_lock.create_job(Some("/project3".to_string()), Some(2));
        let job3 = job_mgr_lock.jobs.get(&job3_id).unwrap();
        let display3 = format!("{}", job3);
        assert!(display3.contains("@job_3.toml"));
        
        drop(job_mgr_lock);
    }

    #[test]
    fn test_concurrent_ui_operations() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Create jobs concurrently and test UI state management
        let job_mgr = store.job_mgr.clone();
        
        let handles: Vec<_> = (0..5).map(|i| {
            let job_mgr = job_mgr.clone();
            std::thread::spawn(move || {
                let mut job_mgr_lock = job_mgr.lock().unwrap();
                job_mgr_lock.create_job(Some(format!("/project_{}", i)), Some(i % 2))
            })
        }).collect();
        
        let job_ids: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        
        // Test that UI can handle multiple jobs
        for job_id in job_ids {
            states.job_states.set_selected_job_id(Some(job_id));
            assert_eq!(states.job_states.get_current_job_id(), job_id);
        }
        
        // Verify all jobs exist
        let job_mgr_lock = job_mgr.lock().unwrap();
        assert_eq!(job_mgr_lock.jobs.len(), 5);
    }

    #[test]
    fn test_keyboard_shortcuts() {
        let temp_dir = TempDir::new().unwrap();
        let mut store = create_test_store();
        let mut states = States::default();
        
        // Test job list shortcuts
        job::list::handle_key(&create_key_event(KeyCode::Char('N')), &mut states, &store);
        assert_eq!(states.job_states.list.tab_action, job::list::Tab::New);
        
        job::list::handle_key(&create_key_event(KeyCode::Char('n')), &mut states, &store);
        assert_eq!(states.job_states.list.tab_action, job::list::Tab::New);
        
        // Test job detail shortcuts
        job::detail::handle_key(&create_key_event(KeyCode::Char('P')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Pod);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('F')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Files);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('R')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Run);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('A')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Cancel);
        
        job::detail::handle_key(&create_key_event(KeyCode::Char('C')), &mut states, &mut store);
        assert_eq!(states.job_states.detail.tab_action, job::detail::Tab::Chat);
    }
}