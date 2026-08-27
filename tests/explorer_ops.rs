use std::collections::HashMap;
use std::path::{Path, PathBuf};

use splitype::explorer::state::state::{
    ExplorerEditState, ExplorerEntryId, ExplorerFilenameEditor, build_explorer_rows,
};
use splitype::explorer::state::undo::{
    ExplorerChange, ExplorerUndoHistory, execute_explorer_change_inverse,
    explorer_change_destination,
};

#[test]
fn test_build_explorer_rows_handles_empty_worktree_safely() {
    let edit = ExplorerEditState {
        root: 0,
        parent_id: Some(ExplorerEntryId(999)),
        target_id: None,
        is_dir: false,
        depth: 1,
        path: PathBuf::from("/empty"),
        validation: None,
        filename: ExplorerFilenameEditor::default(),
        previously_selected: None,
        processing: false,
    };
    // Empty trees list
    let rows = build_explorer_rows(&[], &HashMap::new(), Some(&edit));
    assert!(rows.is_empty());
}

#[test]
fn test_batch_undo_record_and_destination() {
    let mut history = ExplorerUndoHistory::default();
    let change1 = ExplorerChange::Created {
        path: PathBuf::from("/p/1.md"),
        is_dir: false,
    };
    let change2 = ExplorerChange::Created {
        path: PathBuf::from("/p/2.md"),
        is_dir: false,
    };
    let batch = ExplorerChange::Batch(vec![change1, change2]);
    assert_eq!(
        explorer_change_destination(&batch),
        Some(Path::new("/p/2.md"))
    );

    history.record(batch);
    assert!(history.can_undo());
    assert!(!history.can_redo());
}

#[test]
fn test_nested_path_creation_and_batch_rollback() {
    let temp_dir = std::env::temp_dir().join(format!("splitype-test-nested-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let nested_file = temp_dir.join("a").join("b").join("c.md");
    let parent = nested_file.parent().unwrap();
    std::fs::create_dir_all(parent).unwrap();
    std::fs::write(&nested_file, "hello").unwrap();

    let change = ExplorerChange::Created {
        path: nested_file.clone(),
        is_dir: false,
    };

    assert!(nested_file.exists());
    let _ = execute_explorer_change_inverse(&change);
    // Inverse deletes / trashes the created file
    assert!(!nested_file.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}
