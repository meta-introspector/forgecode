use forge_select::ForgeWidget;

fn main() {
    // Simulate a file change preview
    let preview = r#"=== File Changes Preview ===
File: /home/user/project/src/main.rs
Change Type: Modified

--- Old Content ---
fn main() {
    println!("Hello, world!");
    let x = 5;
    let y = 10;
    println!("Sum: {}", x + y);
}

+++ New Content ---
fn main() {
    println!("Hello, world!");
    let x = 10;
    let y = 20;
    let z = x * y;
    println!("Sum: {}", x + y);
    println!("Product: {}", z);
}

=== End of Preview ==="#;

    // Permission details to review
    let permissions = r#"Current Permissions Required:
Read: /home/user/project/src/main.rs (existing file)
Write: /home/user/project/src/main.rs (create/overwrite)
Execute: terminal commands (none required)

Permission Policy: ALLOW (user has write access to this file)"#;

    // Pattern rules for this operation
    let patterns = r#"Matching Patterns:
- *.rs files: allowed with preview
- src/**/*.rs: auto-approve if under 100 lines changed
- This file matches: src/**/*.rs (auto-approve eligible)"#;

    let result = ForgeWidget::confirm("Apply these changes?")
        .preview(preview.to_string())
        .permissions(permissions.to_string())
        .patterns(patterns.to_string())
        .actions(vec![
            "Approve".to_string(),
            "Approve Always".to_string(),
            "Review Permissions".to_string(),
            "Edit Patterns".to_string(),
        ])
        .dry_run()
        .prompt();

    match result {
        Ok(Some(true)) => println!("Approved!"),
        Ok(Some(false)) => println!("Rejected"),
        Ok(None) => println!("Cancelled"),
        Err(e) => eprintln!("Error: {}", e),
    }
}