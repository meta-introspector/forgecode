use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use serde_json;

/// Task manager plugin for GOAP-based task planning
/// Exports ZOS plugin interface and task management functions

// Global state for task management
static TASKS: Mutex<Vec<Task>> = Mutex::new(Vec::new());
static NEXT_ID: Mutex<i32> = Mutex::new(1);

// GOAP state
static GOAP_PLANNER: Mutex<Option<GoapPlanner>> = Mutex::new(None);

// GOAP context for planning
struct GoapContext {
    tasks: Vec<Task>,
}
static GOAP_CONTEXT: Mutex<Option<GoapContext>> = Mutex::new(None);

// Update GOAP context when tasks change
fn update_goap_context() {
    let tasks = TASKS.lock().unwrap();
    *GOAP_CONTEXT.lock().unwrap() = Some(GoapContext { tasks: tasks.clone() });
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Task {
    id: i32,
    description: String,
    status: String, // pending, in_progress, completed, cancelled
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoapAction {
    name: String,
    preconditions: Vec<String>,
    effects: Vec<String>,
    cost: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GoapPlanner {
    actions: Vec<GoapAction>,
}

impl GoapPlanner {
    fn new() -> Self {
        // Define basic actions for task management
        let actions = vec![
            GoapAction {
                name: "analyze_task".to_string(),
                preconditions: vec!["task_exists".to_string()],
                effects: vec!["task_analyzed".to_string()],
                cost: 1,
            },
            GoapAction {
                name: "execute_task".to_string(),
                preconditions: vec!["task_analyzed".to_string()],
                effects: vec!["task_completed".to_string()],
                cost: 2,
            },
            GoapAction {
                name: "create_task".to_string(),
                preconditions: vec![],
                effects: vec!["task_exists".to_string()],
                cost: 1,
            },
        ];
        Self { actions }
    }

    fn plan(&self, goal: &str) -> Vec<String> {
        // Simple placeholder planner that returns a static plan
        // In a real implementation, this would perform GOAP search
        if goal.contains("complete") {
            vec![
                "create_task".to_string(),
                "analyze_task".to_string(),
                "execute_task".to_string(),
            ]
        } else if goal.contains("list") {
            vec!["list_tasks".to_string()]
        } else {
            vec!["analyze_task".to_string()]
        }
    }
}

// ZOS Plugin Interface Functions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn zos_plugin_name() -> *const c_char {
    CString::new("task_manager_plugin").unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn zos_plugin_init() -> i32 {
    // Initialize GOAP planner
    let mut planner = GOAP_PLANNER.lock().unwrap();
    if planner.is_none() {
        *planner = Some(GoapPlanner::new());
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn zos_plugin_shutdown() -> i32 {
    // Cleanup
    let mut tasks = TASKS.lock().unwrap();
    tasks.clear();
    let mut planner = GOAP_PLANNER.lock().unwrap();
    *planner = None;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn zos_plugin_version() -> *const c_char {
    CString::new("0.1.0").unwrap().into_raw()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn zos_plugin_description() -> *const c_char {
    CString::new("Task Manager Plugin - GOAP-based task planning and management").unwrap().into_raw()
}

// Task Management Functions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn task_manager_add_task(description: *const c_char) -> i32 { unsafe {
    if description.is_null() {
        return -1;
    }
    let desc = CStr::from_ptr(description).to_string_lossy().into_owned();
    let mut tasks = TASKS.lock().unwrap();
    let id = *NEXT_ID.lock().unwrap();
    *NEXT_ID.lock().unwrap() += 1;
    tasks.push(Task {
        id,
        description: desc,
        status: "pending".to_string(),
    });
    update_goap_context();
    id
}}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn task_manager_set_status(task_id: i32, status: *const c_char) -> i32 { unsafe {
    if status.is_null() {
        return -1;
    }
    let status_str = CStr::from_ptr(status).to_string_lossy().into_owned();
    let mut tasks = TASKS.lock().unwrap();
    if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
        task.status = status_str;
        update_goap_context();
        0
    } else {
        -1
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn task_manager_get_tasks_json() -> *mut c_char {
    let tasks = TASKS.lock().unwrap();
    let json = serde_json::to_string(&*tasks).unwrap();
    let c_string = CString::new(json).unwrap();
    c_string.into_raw()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn task_manager_plan_goal(goal: *const c_char) -> *mut c_char { unsafe {
    if goal.is_null() {
        return CString::new("[]").unwrap().into_raw();
    }
    let goal_str = CStr::from_ptr(goal).to_string_lossy().into_owned();
    let planner = GOAP_PLANNER.lock().unwrap();
    let plan = if let Some(ref p) = *planner {
        p.plan(&goal_str)
    } else {
        vec![]
    };
    let json = serde_json::to_string(&plan).unwrap();
    CString::new(json).unwrap().into_raw()
}}

// Free function for strings returned by the plugin
#[unsafe(no_mangle)]
pub unsafe extern "C" fn task_manager_free_string(s: *mut c_char) { unsafe {
    if !s.is_null() {
        CString::from_raw(s);
    }
}}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_management() {
        unsafe {
            zos_plugin_init();
            let desc = CString::new("Test task").unwrap();
            let id = task_manager_add_task(desc.as_ptr());
            assert!(id > 0);
            
            let status = CString::new("completed").unwrap();
            let ret = task_manager_set_status(id, status.as_ptr());
            assert_eq!(ret, 0);
            
            let json_ptr = task_manager_get_tasks_json();
            assert!(!json_ptr.is_null());
            let json = CStr::from_ptr(json_ptr).to_string_lossy();
            assert!(json.contains("Test task"));
            assert!(json.contains("completed"));
            task_manager_free_string(json_ptr);
            
            zos_plugin_shutdown();
        }
    }
}