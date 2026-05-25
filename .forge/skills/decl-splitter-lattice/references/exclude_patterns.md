# Decl Splitter & Lattice Generator - Exclude Patterns

## Common Exclude Categories

### E0116: Cross-crate impl
```txt
impl_for_Cause
impl_for_FileStatus
impl_for_Reasoning
impl_for_TerminalContext
impl_for_ChatResponse
impl_for_ChatResponseContent
impl_for_std_fmt_Debug
impl_for_TitleExt
impl_for_TitleFormat
```

### E0277: nom parser round-trip failures
```txt
AttachmentContent
Attachment
DirectoryEntry
FileTag
impl_for_AsRef_std_path_Path
impl_for_AttachmentContent
impl_for_Attachment
impl_for_FileTag
Location
```

### E0432: Unresolved crate:: refs
```txt
ToolCallFull
ToolName
ToolResult
```

### E0433: Missing crate scope
```txt
impl_for_From_Uuid
impl_for_UserId
impl_for_WorkspaceId
impl_for_SnapshotId
impl_for_Snapshot
impl_for_SearchParams
impl_for_CodeBase
```

### E0616: Private field access
```txt
NodeId
impl_for_NodeId
```

### E0210: Type parameter constraint
```txt
impl_for_LineNumbers
impl_for_GroupByKey_K_V
```

### Transitive (depends on excluded type)
```txt
# These often come up when their dependencies are excluded
CodeBase
CodebaseQueryResult
CodebaseSearchResults
CodeSearchQuery
FileRef
FileUploadInfo
FileUploadResponse
GitInfo
impl_for_FileUploadInfo
impl_for_FileUploadResponse
impl_for_From_WorkspaceAuth
impl_for_ToolOrder
impl_for_WorkspaceAuth
MigrationResult
ModelConfig
ModelTestResult
Node
Note
SearchParams
SyncProgress
ToolOrder
Usage
UserId
WorkspaceAuth
WorkspaceInfo
```

## When to Exclude

1. **Cross-crate impls**: When a type is defined in one crate but impls are in another
2. **nom round-trip**: When syn's tokenization of complex nom parsers doesn't work
3. **Missing types**: When inner types weren't split into their own decl files
4. **Private fields**: When tuple struct fields are private across crate boundaries
5. **Transitive**: When a decl depends on an excluded type (process continues iteratively)

## Exclude Strategy

- Start with category-based excludes
- Run `cargo check` and collect failing decls
- Add failing decls to exclude file
- Regenerate and repeat
- Stop when clean build achieved or too many excludes needed
