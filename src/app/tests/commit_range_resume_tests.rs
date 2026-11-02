//! Resuming a commit-range review after the range is rewritten.
//!
//! These drive `App::load_or_create_commit_range_session` — the loader both the
//! `-r` startup path and the inline commit selector use — so a change to the
//! adoption rules has to keep both entry points working.

use crate::app::*;
use crate::model::FileStatus;
use crate::model::diff_types::commit_message_path;
use crate::vcs::traits::{ResolvedRevisionRange, RevisionDiffTarget, VcsType};
use std::path::PathBuf;

use super::TestReviewsDir;

/// Boundary commit that the reviewed range sits on.
const BASE: &str = "1111111111111111111111111111111111111111";
const OTHER_BASE: &str = "2222222222222222222222222222222222222222";

fn vcs_info(root: &Path) -> VcsInfo {
    VcsInfo {
        root_path: root.to_path_buf(),
        head_commit: String::new(),
        branch_name: None,
        vcs_type: VcsType::Git,
    }
}

fn diff_file(path: &str, content_hash: u64) -> DiffFile {
    DiffFile {
        old_path: Some(PathBuf::from(path)),
        new_path: Some(PathBuf::from(path)),
        status: FileStatus::Modified,
        hunks: Vec::new(),
        is_binary: false,
        is_too_large: false,
        is_commit_message: false,
        content_hash,
    }
}

fn message_file(short_id: &str, content_hash: u64) -> DiffFile {
    DiffFile {
        old_path: None,
        new_path: Some(commit_message_path(short_id)),
        status: FileStatus::Added,
        hunks: Vec::new(),
        is_binary: false,
        is_too_large: false,
        is_commit_message: true,
        content_hash,
    }
}

fn commit(id: char, count: usize) -> String {
    std::iter::repeat_n(id, count).collect()
}

/// Persist a reviewed session for a range that a later review replaces.
fn seed_reviewed_session(
    info: &VcsInfo,
    commit_ids: &[String],
    base: Option<&str>,
    files: &[DiffFile],
) -> ReviewSession {
    let mut session = ReviewSession::new(
        info.root_path.clone(),
        commit_ids.last().cloned().unwrap_or_default(),
        info.branch_name.clone(),
        SessionDiffSource::CommitRange,
    );
    session.commit_range = Some(commit_ids.to_vec());
    session.commit_range_base = base.map(str::to_string);
    for file in files {
        session.add_diff_file(file);
        let path = file.display_path().clone();
        session
            .get_file_mut(&path)
            .expect("registered file")
            .reviewed = true;
    }
    crate::persistence::storage::save_session(&session).expect("save seeded session");
    session
}

fn load(
    info: &VcsInfo,
    commit_ids: &[String],
    base: Option<&str>,
    diff_files: &[DiffFile],
) -> ReviewSession {
    let mut session = App::load_or_create_commit_range_session(info, commit_ids, base, diff_files);
    // Startup registers the range diff before it inserts the commit-message
    // file, so mirror that order here.
    App::register_diff_files(&mut session, diff_files, false);
    session
}

#[test]
fn should_resume_reviewed_files_when_a_rewrite_keeps_their_diff() {
    let _reviews = TestReviewsDir::new();
    let root = tempfile::tempdir().expect("temp repo dir");
    let info = vcs_info(root.path());
    let old_ids = vec![commit('a', 40)];
    let unchanged = diff_file("kept.py", 7);
    let rewritten = diff_file("changed.py", 8);
    let old_message = message_file("aaaaaaa", 9);
    seed_reviewed_session(
        &info,
        &old_ids,
        Some(BASE),
        &[unchanged.clone(), rewritten.clone(), old_message],
    );

    let new_ids = vec![commit('b', 40)];
    let mut session = load(
        &info,
        &new_ids,
        Some(BASE),
        &[unchanged.clone(), diff_file("changed.py", 11)],
    );

    assert_eq!(session.commit_range.as_deref(), Some(new_ids.as_slice()));
    assert_eq!(session.commit_range_base.as_deref(), Some(BASE));
    assert_eq!(session.base_commit, new_ids[0]);
    assert!(
        session
            .get_file_mut(&unchanged.display_path().clone())
            .expect("unchanged file resumed")
            .reviewed,
        "a file whose diff did not change stays reviewed"
    );
    assert!(
        !session
            .get_file_mut(&diff_file("changed.py", 11).display_path().clone())
            .expect("changed file resumed")
            .reviewed,
        "a file whose diff changed is reviewed again"
    );
}

#[test]
fn should_leave_a_rewritten_message_unreviewed() {
    let _reviews = TestReviewsDir::new();
    let root = tempfile::tempdir().expect("temp repo dir");
    let info = vcs_info(root.path());
    let file = diff_file("kept.py", 7);
    seed_reviewed_session(
        &info,
        &[commit('a', 40)],
        Some(BASE),
        &[file.clone(), message_file("aaaaaaa", 9)],
    );

    let new_ids = vec![commit('b', 40)];
    let mut session = load(&info, &new_ids, Some(BASE), std::slice::from_ref(&file));
    session.add_diff_file(&message_file("bbbbbbb", 12));

    assert!(
        !session
            .get_file_mut(&commit_message_path("bbbbbbb"))
            .expect("new message file")
            .reviewed,
        "the message of a rewritten commit starts unreviewed"
    );
    assert_eq!(
        session.reviewed_count(),
        1,
        "only the unchanged file counts as reviewed"
    );
    assert_eq!(
        session.files.len(),
        2,
        "the replaced message file drops out of the session"
    );
}

#[test]
fn should_keep_a_message_whose_commit_survives_the_rewrite() {
    let _reviews = TestReviewsDir::new();
    let root = tempfile::tempdir().expect("temp repo dir");
    let info = vcs_info(root.path());
    let untouched = commit('a', 40);
    let file = diff_file("kept.py", 7);
    let message = message_file("aaaaaaa", 9);
    seed_reviewed_session(
        &info,
        std::slice::from_ref(&untouched),
        Some(BASE),
        &[file.clone(), message.clone()],
    );

    // The range gained a commit, so the reviewed commit and its message stay.
    let new_ids = vec![untouched.clone(), commit('b', 40)];
    let mut session = load(&info, &new_ids, Some(BASE), &[file]);
    session.add_diff_file(&message);

    assert!(
        session
            .get_file_mut(&commit_message_path("aaaaaaa"))
            .expect("message file kept")
            .reviewed,
        "a commit that survived the rewrite keeps its reviewed message"
    );
}

#[test]
fn should_not_resume_a_review_of_a_range_on_another_boundary() {
    let _reviews = TestReviewsDir::new();
    let root = tempfile::tempdir().expect("temp repo dir");
    let info = vcs_info(root.path());
    let file = diff_file("kept.py", 7);
    seed_reviewed_session(
        &info,
        &[commit('a', 40)],
        Some(OTHER_BASE),
        std::slice::from_ref(&file),
    );

    let session = load(
        &info,
        &[commit('b', 40)],
        Some(BASE),
        std::slice::from_ref(&file),
    );

    assert_eq!(
        session.reviewed_count(),
        0,
        "another stack never resumes a review"
    );
    assert_eq!(session.commit_range_base.as_deref(), Some(BASE));
}

#[test]
fn should_not_resume_without_a_known_boundary() {
    let _reviews = TestReviewsDir::new();
    let root = tempfile::tempdir().expect("temp repo dir");
    let info = vcs_info(root.path());
    let file = diff_file("kept.py", 7);
    seed_reviewed_session(&info, &[commit('a', 40)], None, std::slice::from_ref(&file));

    let session = load(&info, &[commit('b', 40)], None, std::slice::from_ref(&file));

    assert_eq!(
        session.reviewed_count(),
        0,
        "a range with no boundary cannot identify a predecessor"
    );
    assert_eq!(session.commit_range_base, None);
}

#[test]
fn should_return_a_resumed_session_under_a_new_identity() {
    let _reviews = TestReviewsDir::new();
    let root = tempfile::tempdir().expect("temp repo dir");
    let info = vcs_info(root.path());
    let file = diff_file("kept.py", 7);
    let seeded = seed_reviewed_session(
        &info,
        &[commit('a', 40)],
        Some(BASE),
        std::slice::from_ref(&file),
    );

    let session = load(&info, &[commit('b', 40)], Some(BASE), &[file]);

    assert_ne!(
        session.id, seeded.id,
        "a resumed review is its own session, not the superseded one"
    );
}

/// Boundary that the loader records for a range.
struct ParentVcs {
    info: VcsInfo,
    parent: Option<String>,
}

impl VcsBackend for ParentVcs {
    fn info(&self) -> &VcsInfo {
        &self.info
    }

    fn get_working_tree_diff(&self, _highlighter: &SyntaxHighlighter) -> Result<Vec<DiffFile>> {
        Err(TuicrError::NoChanges)
    }

    fn fetch_context_lines(
        &self,
        _file_path: &Path,
        _file_status: FileStatus,
        _ref_commit: Option<&str>,
        _start_line: u32,
        _end_line: u32,
    ) -> Result<Vec<DiffLine>> {
        Ok(Vec::new())
    }

    fn file_line_count(
        &self,
        _file_path: &Path,
        _file_status: FileStatus,
        _ref_commit: Option<&str>,
    ) -> Result<u32> {
        Ok(0)
    }

    fn commit_parent(&self, _id: &str) -> Result<Option<String>> {
        Ok(self.parent.clone())
    }
}

fn parent_vcs(parent: Option<&str>) -> ParentVcs {
    ParentVcs {
        info: vcs_info(std::path::Path::new("/tmp")),
        parent: parent.map(str::to_string),
    }
}

#[test]
fn should_prefer_the_expression_boundary_over_the_parent() {
    let ids = vec![commit('a', 40)];
    let range = ResolvedRevisionRange::from_commit_ids(
        &ids,
        RevisionDiffTarget::Explicit {
            base: Some(BASE.to_string()),
            head: ids[0].clone(),
        },
    );

    assert_eq!(
        App::commit_range_base(&parent_vcs(Some(OTHER_BASE)), &range).as_deref(),
        Some(BASE),
        "a revision expression names its own boundary"
    );
}

#[test]
fn should_use_the_oldest_parent_for_a_selected_commit_list() {
    let ids = vec![commit('a', 40), commit('b', 40)];
    let range = ResolvedRevisionRange::from_commit_ids(&ids, RevisionDiffTarget::CommitList);

    assert_eq!(
        App::commit_range_base(&parent_vcs(Some(BASE)), &range).as_deref(),
        Some(BASE)
    );
    assert_eq!(
        App::commit_range_base(&parent_vcs(None), &range),
        None,
        "a backend that cannot report a parent records no boundary"
    );
}

#[test]
fn should_record_no_boundary_for_an_empty_range() {
    let ids: Vec<String> = Vec::new();
    let range = ResolvedRevisionRange::from_commit_ids(&ids, RevisionDiffTarget::CommitList);

    assert_eq!(
        App::commit_range_base(&parent_vcs(Some(BASE)), &range),
        None
    );
}
