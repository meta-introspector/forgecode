# GOAP Planning Skill

## Overview
Goal-Oriented Action Planning (GOAP) implementation within the ForgeCode ecosystem. This skill enables the creation, management, and execution of planning workflows that integrate with ZOS plugins, formal verification, and skill suggestion systems.

## Planning Workflow

### 1. Request Skill Suggestions
```rust
// Example: Get skills for goap-planning context
unsafe {
    let goal_ptr = CString::new("goap-planning").unwrap();
    let skills_ptr = zos_plugin_suggest_skills(goal_ptr.as_ptr());
    let skills_json = CStr::from_ptr(skills_ptr).to_str().unwrap();
    let skills: Vec<String> = serde_json::from_str(skills_json).unwrap();
    // Expected: ["goap-core", "goap-optimizer", "goap-verifier"]
}
```

### 2. Generate Action Plan
```rust
// Example: Plan actions for "find new work"
unsafe {
    let goal_ptr = CString::new("find new work").unwrap();
    let plan_ptr = task_manager_plan_goal(goal_ptr.as_ptr());
    let plan_json = CStr::from_ptr(plan_ptr).to_str().unwrap();
    let actions: Vec<String> = serde_json::from_str(plan_json).unwrap();
    // Expected: ["analyze_task", "create_task", "execute_task", "list_tasks"]
}
```

## Implementation

### Skill Context Mappings
| Context | Skills Generated |
|---------|-----------------|
| goap-planning | ["goap-core", "goap-optimizer", "goap-verifier"] |
| task-planning | ["zkprologml-goap", "zkprologml-decision-tree", "zkprologml-plan-optimization"] |
| dasl-indexing | ["dasl-index", "ipld-car-shmem", "graphify"] |
| lean4-verification | ["lean4", "lean4-repl-server", "proof-cid"] |

### Core Concepts

#### Actions
- **Atomic Units**: Each action is indivisible
- **Preconditions**: Conditions that must be true before execution
- **Effects**: Changes to world state after execution
- **Cost**: Resource expenditure for execution

#### Plans
- **Goal**: Desired end state
- **Actions**: Ordered sequence achieving goal
- **Cost**: Total execution cost
- **Proof**: Formal verification evidence

## Integration

### With Lean4 Verification
```lean
-- Example theorem proving plan correctness
theorem plan_correctness (plan : Plan) :
  valid (plan.actions) → ∃ result, execute_plan plan = result :=
begin
  -- Proof obligations
end
```

### With ZKPrologML
```prolog
% Example Prolog rule for skill matching
goal_matches_skill(Goal, Skill) :-
    member(Goal, [goap-planning, task-planning]),
    member(Skill, [zkprologml-goap, zkprologml-decision-tree]).
```

### With Minizinc Optimization
```minizinc
% Minimize plan execution cost
solve minimize sum(action in plan.actions)(action.cost) +
           sum(task in plan.tasks)(task.priority * task.weight)
```

## Usage

### Command Line
```bash
# Plan for a specific goal
cargo run -p forge_infra -- zos_plugin_bridge --plan-goal "find new work"

# Suggest skills for planning
cargo run -p forge_infra -- zos_plugin_bridge --suggest-skills "goap-planning"

# Verify a plan
cargo run -p forge_infra -- zos_plugin_bridge --verify-plan "plan.json"
```

### Rust API
```rust
use forge_task_manager_plugin::{plan_goal, suggest_skills};

fn create_plan(goal: &str) -> anyhow::Result<Plan> {
    let skills = suggest_skills("goap-planning")?;
    let plan = plan_goal(goal)?;
    Ok(plan)
}
```

## Testing

### Unit Tests
```rust
#[test]
fn test_suggest_skills() {
    let skills = suggest_skills("task-planning").unwrap();
    assert!(skills.contains(&"zkprologml-goap".to_string()));
}

#[test]
fn test_plan_generation() {
    let plan = plan_goal("find new work").unwrap();
    assert!(!plan.actions.is_empty());
}
```

### Integration Tests
```bash
cargo test -p forge_task_manager_plugin -- --nocapture
```

## References

- GOAP Algorithm: `https://en.wikipedia.org/wiki/Goal-oriented_action_planning`
- Task Manager Plugin: `crates/forge_task_manager_plugin/`
- ZKPrologML Plugin: `crates/zkprologml_plugin/`
- ZOS Bridge: `crates/forge_infra/src/zos_plugin_bridge.rs`

---

*This skill documents GOAP planning integration within ForgeCode.*