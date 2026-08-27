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

#[test]
fn test_circular_drag_and_copy_protection() {
    let temp_dir = std::env::temp_dir().join(format!("splitype-test-circ-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let parent_dir = temp_dir.join("folder_a");
    let child_dir = parent_dir.join("folder_b");
    std::fs::create_dir_all(&child_dir).unwrap();

    // 1. Attempt to copy parent_dir into child_dir via execute_entry_ops -> must be skipped
    let changes_copy = splitype::explorer::state::utils::execute_entry_ops(
        &[parent_dir.clone()],
        &child_dir,
        false,
        false,
    );
    assert!(changes_copy.is_empty(), "Circular copy must be prevented");

    // 2. Attempt to move parent_dir into child_dir via execute_entry_ops -> must be skipped
    let changes_move = splitype::explorer::state::utils::execute_entry_ops(
        &[parent_dir.clone()],
        &child_dir,
        true,
        false,
    );
    assert!(changes_move.is_empty(), "Circular move must be prevented");

    // 3. Directly call copy_dir_all into descendant -> must return error
    let res = splitype::explorer::state::utils::copy_dir_all(&parent_dir, &child_dir);
    assert!(res.is_err(), "copy_dir_all into descendant must return error");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_nested_dir_creation_and_clean_undo() {
    let temp_dir = std::env::temp_dir().join(format!("splitype-test-undo-clean-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let dir_a = temp_dir.join("alpha");
    let dir_b = dir_a.join("beta");
    let leaf_file = dir_b.join("doc.md");

    // Simulate batch creation of missing parent dirs + leaf file
    std::fs::create_dir_all(&dir_b).unwrap();
    std::fs::write(&leaf_file, "# Document Content").unwrap();

    let batch = ExplorerChange::Batch(vec![
        ExplorerChange::DirCreated(dir_a.clone()),
        ExplorerChange::DirCreated(dir_b.clone()),
        ExplorerChange::Created {
            path: leaf_file.clone(),
            is_dir: false,
        },
    ]);

    assert!(leaf_file.exists());
    assert!(dir_b.exists());
    assert!(dir_a.exists());

    // Undo the batch change
    let _ = execute_explorer_change_inverse(&batch);

    // Assert that the file is gone AND the empty intermediate directories are cleanly deleted!
    assert!(!leaf_file.exists());
    assert!(!dir_b.exists());
    assert!(!dir_a.exists());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_disambiguated_paste_and_duplicate_naming() {
    let temp_dir = std::env::temp_dir().join(format!("splitype-test-dup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let doc = temp_dir.join("report.md");
    std::fs::write(&doc, "Initial report").unwrap();

    // 1. First duplicate should produce "report copy.md"
    let (cand1, range1) = splitype::explorer::state::utils::disambiguated_paste_path(&doc, &temp_dir);
    assert_eq!(cand1.file_name().unwrap(), "report copy.md");
    assert_eq!(range1, Some(0.."report copy".len()));
    std::fs::write(&cand1, "Copy 1").unwrap();

    // 2. Second duplicate should produce "report copy 1.md"
    let (cand2, range2) = splitype::explorer::state::utils::disambiguated_paste_path(&doc, &temp_dir);
    assert_eq!(cand2.file_name().unwrap(), "report copy 1.md");
    assert_eq!(range2, Some(0.."report copy 1".len()));
    std::fs::write(&cand2, "Copy 2").unwrap();

    // 3. Third duplicate should produce "report copy 2.md"
    let (cand3, _) = splitype::explorer::state::utils::disambiguated_paste_path(&doc, &temp_dir);
    assert_eq!(cand3.file_name().unwrap(), "report copy 2.md");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_undo_and_redo_move_operation() {
    let temp_dir = std::env::temp_dir().join(format!("splitype-test-undo-move-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&temp_dir);
    std::fs::create_dir_all(&temp_dir).unwrap();

    let sub_dir = temp_dir.join("sub");
    std::fs::create_dir_all(&sub_dir).unwrap();

    let orig = temp_dir.join("file.txt");
    std::fs::write(&orig, "Move test").unwrap();

    // Move file.txt -> sub/file.txt
    let changes = splitype::explorer::state::utils::execute_entry_ops(
        &[orig.clone()],
        &sub_dir,
        true,
        false,
    );
    assert_eq!(changes.len(), 1);
    let dest = sub_dir.join("file.txt");
    assert!(!orig.exists());
    assert!(dest.exists());

    // Undo the move (inverse)
    for change in changes.iter().rev() {
        let _ = execute_explorer_change_inverse(change);
    }
    assert!(orig.exists());
    assert!(!dest.exists());
    assert_eq!(std::fs::read_to_string(&orig).unwrap(), "Move test");

    // Redo the move (forward)
    for change in &changes {
        let _ = splitype::explorer::state::undo::execute_explorer_change(change);
    }
    assert!(!orig.exists());
    assert!(dest.exists());
    assert_eq!(std::fs::read_to_string(&dest).unwrap(), "Move test");

    let _ = std::fs::remove_dir_all(&temp_dir);
}

