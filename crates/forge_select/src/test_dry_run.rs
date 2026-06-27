use forge_select::ForgeWidget;

fn main() {
    // Example 1: Dry run mode for bench testing
    println!("=== Example 1: Dry Run Mode ===");
    let confirm_result = ForgeWidget::confirm("Request permission to access data?")
        .view_more("This operation will:
- Read sensitive data
- Write to output
- Execute external commands
- Access network resources")
        .actions(vec![
            "Approve".to_string(),
            "Approve Always".to_string(),
            "Review Permissions".to_string(),
            "Edit Patterns".to_string()
        ])
        .dry_run()
        .prompt()
        .expect("Failed to prompt");
    
    println!("Result: {:?}", confirm_result);
    
    // Example 2: Normal mode with view more
    println!("\n=== Example 2: Normal Mode with View More ===");
    let confirm_result = ForgeWidget::confirm("Proceed with operation?")
        .view_more("This operation will:
- Create files
- Modify settings
- Run commands")
        .actions(vec![
            "Confirm".to_string(),
            "Review Details".to_string(),
            "Cancel".to_string()
        ])
        .prompt()
        .expect("Failed to prompt");
    
    println!("Result: {:?}", confirm_result);
}
