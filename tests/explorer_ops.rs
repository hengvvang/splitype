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

#[test]
fn test_move_file_to_subfolder_and_back_to_parent() {
    let temp_dir = std::env::temp_dir().join(format!("splitype-test-move-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let sub_dir = temp_dir.join("subfolder");
    std::fs::create_dir_all(&sub_dir).unwrap();

    let root_file = temp_dir.join("test_note.md");
    std::fs::write(&root_file, "# Root Note").unwrap();

    // 1. Move root_file into sub_dir (is_cut = true)
    let changes = splitype::explorer::state::utils::execute_entry_ops(
        &[root_file.clone()],
        &sub_dir,
        true,
        false,
    );
    assert_eq!(changes.len(), 1);
    let target_file = sub_dir.join("test_note.md");
    assert!(!root_file.exists());
    assert!(target_file.exists());
    assert_eq!(std::fs::read_to_string(&target_file).unwrap(), "# Root Note");

    // 2. Move target_file back to root temp_dir (parent move)
    let changes2 = splitype::explorer::state::utils::execute_entry_ops(
        &[target_file.clone()],
        &temp_dir,
        true,
        false,
    );
    assert_eq!(changes2.len(), 1);
    assert!(!target_file.exists());
    assert!(root_file.exists());
    assert_eq!(std::fs::read_to_string(&root_file).unwrap(), "# Root Note");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

