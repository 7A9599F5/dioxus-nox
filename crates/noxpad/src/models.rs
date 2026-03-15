//! Data models and seed data for NoxPad.

#[derive(Clone)]
pub(crate) struct Note {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) tags: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct FolderNode {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) note_indices: Vec<usize>,
    pub(crate) collapsed: bool,
}

pub(crate) fn seed_notes() -> Vec<Note> {
    vec![
        Note {
            id: "rust-ownership".into(),
            title: "Rust Ownership".into(),
            content: "# Rust Ownership\n\nRust's ownership system enables memory safety without a garbage collector.\n\n## The Rules\n\n- Each value has a single *owner*\n- When the owner goes out of scope, the value is dropped\n\n## Borrowing\n\nBorrowing lets you reference data without taking ownership.\n\n```rust\nfn print_len(s: &String) {\n    println!(\"Length: {}\", s.len());\n}\n```\n\n- [ ] Review lifetimes chapter\n- [ ] Practice with custom types".into(),
            tags: vec!["rust".into(), "memory".into(), "ownership".into()],
        },
        Note {
            id: "wasm-perf".into(),
            title: "WASM Performance".into(),
            content: "# WASM Performance\n\nWebAssembly runs at near-native speed in the browser.\n\n## Key Techniques\n\n### Minimize JS-WASM Boundary Crossings\n\nEach call across the boundary has overhead. Batch operations where possible.\n\n### Use Linear Memory\n\nPrefer `Vec<u8>` over complex allocations when passing data to JS.\n\n---\n\n> The fastest code is code that doesn't run.".into(),
            tags: vec!["wasm".into(), "performance".into(), "rust".into()],
        },
        Note {
            id: "meeting-notes".into(),
            title: "Meeting Notes".into(),
            content: "# Meeting Notes - Component API Review\n\n## Attendees\n\n@Alice @Bob @Carol\n\n## Action Items\n\n- [ ] Alice: update cmdk signal pattern\n- [ ] Bob: write migration guide\n- [ ] Carol: add E2E tests for dnd\n\n## Notes\n\nDecided to use `data-state` attributes consistently across all crates.".into(),
            tags: vec!["meeting".into(), "planning".into()],
        },
        Note {
            id: "reading-list".into(),
            title: "Reading List".into(),
            content: "# Reading List\n\n## In Progress\n\n- *Programming Rust* - Blandy & Orendorff\n- *The Rust Programming Language* - Klabnik & Nichols\n\n## Backlog\n\n- *Crafting Interpreters* - Robert Nystrom\n- *Database Internals* - Alex Petrov\n\n## Completed\n\n- *The Art of Problem Solving*\n\n#rust #books #learning".into(),
            tags: vec!["books".into(), "learning".into(), "rust".into()],
        },
    ]
}

pub(crate) fn seed_folders() -> Vec<FolderNode> {
    vec![
        FolderNode {
            id: "inbox".into(),
            name: "Inbox".into(),
            note_indices: vec![0, 2],
            collapsed: false,
        },
        FolderNode {
            id: "engineering".into(),
            name: "Engineering".into(),
            note_indices: vec![1],
            collapsed: false,
        },
        FolderNode {
            id: "reference".into(),
            name: "Reference".into(),
            note_indices: vec![3],
            collapsed: false,
        },
    ]
}
