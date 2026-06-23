//! Build script — keeps the embedded `RUSTF_SKILL.md` shipped to new
//! projects in sync with the canonical skill at `.claude/skills/rustf/SKILL.md`.
//!
//! Why: the rust-embed crate reads the file at compile time from
//! `templates/project/claude_skills/rustf/SKILL.md`. Maintaining a hand-
//! synced copy is bug-prone. This script copies the canonical doc into
//! the template tree before `rust-embed` runs.

use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let canonical = manifest_dir.join("../.claude/skills/rustf/SKILL.md");
    let template = manifest_dir
        .join("templates/project/claude_skills/rustf/SKILL.md");

    println!("cargo:rerun-if-changed={}", canonical.display());

    if !canonical.exists() {
        // Doc not in the expected location — emit a warning but don't
        // fail the build (helps if the crate is unpacked standalone).
        println!(
            "cargo:warning=.claude/skills/rustf/SKILL.md not found at {} — \
             template SKILL.md not refreshed",
            canonical.display()
        );
        return;
    }

    if let Some(parent) = template.parent() {
        std::fs::create_dir_all(parent).expect("create template parent dir");
    }

    let canonical_bytes = std::fs::read(&canonical).expect("read canonical SKILL.md");
    let needs_write = match std::fs::read(&template) {
        Ok(existing) => existing != canonical_bytes,
        Err(_) => true,
    };
    if needs_write {
        std::fs::write(&template, &canonical_bytes).expect("write template SKILL.md");
    }
}
