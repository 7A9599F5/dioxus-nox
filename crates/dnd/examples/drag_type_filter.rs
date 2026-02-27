//! DragType filtering example with Sortable pattern
//!
//! Demonstrates how to combine Sortable reordering with DragType filtering.
//! Items have multiple types - they are "sortable" (for reordering within lists)
//! AND have content types (image, document, video) for filtering which
//! containers accept them.
//!
//! This example uses the SortableGroup + SortableContext pattern with:
//! - SortableItem with `content_types` prop for additional content type
//! - SortableContext with `accepts` prop for type-filtered containers
//!
//! Run with: dx serve --example drag_type_filter

use dioxus::prelude::*;
use dioxus_nox_dnd::{
    DragId, DragOverlay, DragType, MoveEvent, ReorderEvent, SortableContext, SortableGroup,
    SortableItem, FUNCTIONAL_STYLES, THEME_STYLES,
};

// Container ID constants
const MEDIA_FOLDER_ID: &str = "media-folder";
const DOCS_FOLDER_ID: &str = "docs-folder";
const TRASH_ID: &str = "trash";
const SOURCE_ID: &str = "source";

// DragType constants for content types
const TYPE_IMAGE: &str = "image";
const TYPE_DOCUMENT: &str = "document";
const TYPE_VIDEO: &str = "video";

fn main() {
    dioxus::launch(app);
}

/// A file item with a stable ID and content type
#[derive(Clone, Debug, PartialEq)]
struct FileItem {
    id: String,
    name: String,
    file_type: FileType,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum FileType {
    Image,
    Document,
    Video,
}

impl FileItem {
    fn drag_id(&self) -> DragId {
        DragId::new(&self.id)
    }

    /// Get the content type for filtering (NOT the sortable type)
    fn content_type(&self) -> DragType {
        match self.file_type {
            FileType::Image => DragType::new(TYPE_IMAGE),
            FileType::Document => DragType::new(TYPE_DOCUMENT),
            FileType::Video => DragType::new(TYPE_VIDEO),
        }
    }

    fn icon(&self) -> &'static str {
        match self.file_type {
            FileType::Image => "🖼️",
            FileType::Document => "📄",
            FileType::Video => "🎬",
        }
    }

    fn color_class(&self) -> &'static str {
        match self.file_type {
            FileType::Image => "file-type-image",
            FileType::Document => "file-type-document",
            FileType::Video => "file-type-video",
        }
    }
}

/// A sortable folder component that accepts specific file types
///
/// Uses SortableContext with `accepts` prop for type filtering
/// and SortableItem with `types` prop for multi-type items
#[component]
fn SortableFileFolder(
    id: DragId,
    title: String,
    items: Signal<Vec<FileItem>>,
    accepts: Vec<DragType>,
    icon: String,
    folder_class: String,
) -> Element {
    // Build accepts description for UI
    let accepts_desc = if accepts.is_empty() {
        "All types".to_string()
    } else {
        let types: Vec<&str> = accepts
            .iter()
            .map(|t| match t.0.as_str() {
                TYPE_IMAGE => "Images",
                TYPE_DOCUMENT => "Documents",
                TYPE_VIDEO => "Videos",
                "sortable" => "Sortable",
                _ => "Unknown",
            })
            .collect();
        types.join(", ")
    };

    // Read items once for efficiency (avoiding multiple signal reads)
    let items_value = items.read();

    // Get item IDs for the SortableContext
    let item_ids: Vec<DragId> = items_value.iter().map(|f| f.drag_id()).collect();

    rsx! {
        div { class: "folder-wrapper",
            div {
                class: "folder-header {folder_class}",
                span { class: "folder-icon", "{icon}" }
                h2 { "{title}" }
            }
            div { class: "accepts-info", "Accepts: {accepts_desc}" }

            // Use SortableContext with accepts prop for type filtering
            SortableContext {
                id: id,
                items: item_ids,
                accepts: accepts,

                div { class: "folder-contents",
                    for file in items_value.iter() {
                        // Use SortableItem with `content_types` prop to add content type
                        // Items are automatically "sortable" + their content type
                        SortableItem {
                            key: "{file.id}",
                            id: file.drag_id(),
                            content_types: vec![file.content_type()], // Adds "image"/"document"/"video" to "sortable"

                            div {
                                class: "file-item {file.color_class()}",
                                span { class: "file-icon", "{file.icon()}" }
                                span { class: "file-name", "{file.name}" }
                            }
                        }
                    }

                    if items_value.is_empty() {
                        div { class: "empty-state", "Drop files here" }
                    }
                }
            }
        }
    }
}

fn app() -> Element {
    // Source files (mixed types)
    let source = use_signal(|| {
        vec![
            FileItem {
                id: "img1".to_string(),
                name: "vacation.jpg".to_string(),
                file_type: FileType::Image,
            },
            FileItem {
                id: "doc1".to_string(),
                name: "report.pdf".to_string(),
                file_type: FileType::Document,
            },
            FileItem {
                id: "vid1".to_string(),
                name: "tutorial.mp4".to_string(),
                file_type: FileType::Video,
            },
            FileItem {
                id: "img2".to_string(),
                name: "screenshot.png".to_string(),
                file_type: FileType::Image,
            },
            FileItem {
                id: "doc2".to_string(),
                name: "notes.txt".to_string(),
                file_type: FileType::Document,
            },
        ]
    });

    // Media folder (accepts images and videos only)
    let media: Signal<Vec<FileItem>> = use_signal(Vec::new);

    // Documents folder (accepts documents only)
    let docs: Signal<Vec<FileItem>> = use_signal(Vec::new);

    // Trash (accepts all types)
    let trash: Signal<Vec<FileItem>> = use_signal(Vec::new);

    // Define containers for the apply() helpers
    let containers = [
        (DragId::new(SOURCE_ID), source),
        (DragId::new(MEDIA_FOLDER_ID), media),
        (DragId::new(DOCS_FOLDER_ID), docs),
        (DragId::new(TRASH_ID), trash),
    ];

    // Clone for second closure
    let containers_for_move = containers.clone();

    rsx! {
        style { {FUNCTIONAL_STYLES} }
        style { {THEME_STYLES} }
        style { {DRAG_TYPE_STYLES} }

        div { class: "container",
            h1 { "Multi-Type Sortable Example" }
            p { class: "instructions",
                "Items are sortable within lists AND type-filtered across containers. "
                "Media folder accepts images/videos. Documents folder accepts documents. "
                "Trash accepts all types. Items can be reordered within each folder."
            }

            // SortableGroup provides shared drag context for cross-container moves
            SortableGroup {
                on_reorder: move |e: ReorderEvent| {
                    // Same-container reorder (just reorder within the list)
                    e.apply(&containers, |f: &FileItem| f.drag_id());
                },
                on_move: move |e: MoveEvent| {
                    // Cross-container move (move item between lists)
                    e.apply(&containers_for_move, |f: &FileItem| f.drag_id());
                },

                div { class: "folders-container",
                    // Source files - accepts all types (empty accepts = all)
                    SortableFileFolder {
                        id: DragId::new(SOURCE_ID),
                        title: "Files".to_string(),
                        items: source,
                        accepts: vec![], // Empty = accepts all
                        icon: "📁".to_string(),
                        folder_class: "folder-files".to_string(),
                    }

                    // Media folder - only accepts images and videos
                    SortableFileFolder {
                        id: DragId::new(MEDIA_FOLDER_ID),
                        title: "Media".to_string(),
                        items: media,
                        accepts: vec![
                            DragType::new(TYPE_IMAGE),
                            DragType::new(TYPE_VIDEO),
                        ],
                        icon: "🎨".to_string(),
                        folder_class: "folder-media".to_string(),
                    }

                    // Documents folder - only accepts documents
                    SortableFileFolder {
                        id: DragId::new(DOCS_FOLDER_ID),
                        title: "Documents".to_string(),
                        items: docs,
                        accepts: vec![DragType::new(TYPE_DOCUMENT)],
                        icon: "📚".to_string(),
                        folder_class: "folder-documents".to_string(),
                    }

                    // Trash - accepts everything
                    SortableFileFolder {
                        id: DragId::new(TRASH_ID),
                        title: "Trash".to_string(),
                        items: trash,
                        accepts: vec![], // Empty = accepts all types
                        icon: "🗑️".to_string(),
                        folder_class: "folder-trash".to_string(),
                    }
                }

                DragOverlay {
                    div { class: "file-item dragging", "Moving file..." }
                }
            }
        }
    }
}

const DRAG_TYPE_STYLES: &str = r#"
    body {
        font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
        padding: 20px;
        background: var(--dxdnd-bg-subtle);
        margin: 0;
    }

    .container {
        max-width: 1200px;
        margin: 0 auto;
    }

    h1 {
        margin-bottom: 8px;
        color: var(--dxdnd-text);
    }

    .instructions {
        color: var(--dxdnd-text-muted);
        margin-bottom: 24px;
        line-height: 1.5;
    }

    .folders-container {
        display: grid;
        grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
        gap: 20px;
    }

    .folder-wrapper {
        background: var(--dxdnd-bg);
        border-radius: 8px;
        overflow: hidden;
        box-shadow: 0 2px 8px rgba(0,0,0,0.1);
    }

    .folder-header {
        display: flex;
        align-items: center;
        gap: 8px;
        padding: 12px 16px;
        border-bottom: 2px solid var(--dxdnd-border);
    }

    /* Folder header color variants - light mode */
    .folder-header.folder-files {
        background: #f5f5f5;
        color: #424242;
    }

    .folder-header.folder-media {
        background: #e8f5e9;
        color: #1b5e20;
    }

    .folder-header.folder-documents {
        background: #fff3e0;
        color: #e65100;
    }

    .folder-header.folder-trash {
        background: #ffebee;
        color: #b71c1c;
    }

    .folder-header h2 {
        font-size: 16px;
        font-weight: 600;
        margin: 0;
        flex: 1;
        color: inherit;
    }

    .folder-icon {
        font-size: 20px;
    }

    .accepts-info {
        padding: 8px 16px;
        font-size: 12px;
        color: var(--dxdnd-text-muted);
        background: rgba(0,0,0,0.02);
        border-bottom: 1px solid rgba(0,0,0,0.05);
    }

    .folder-contents {
        padding: 12px;
        min-height: 200px;
    }

    .file-item {
        display: flex;
        align-items: center;
        gap: 8px;
    }

    /* File type color variants - light mode (higher specificity to override THEME_STYLES) */
    .sortable-item > .file-item.file-type-image {
        background: #e3f2fd;
        color: #1565c0;
    }

    .sortable-item > .file-item.file-type-document {
        background: #fff3e0;
        color: #e65100;
    }

    .sortable-item > .file-item.file-type-video {
        background: #f3e5f5;
        color: #7b1fa2;
    }

    .file-icon {
        font-size: 18px;
    }

    .file-name {
        font-size: 14px;
        font-weight: 500;
        color: inherit;
    }

    /* Dark mode overrides - MUST come after light mode rules */
    @media (prefers-color-scheme: dark) {
        .accepts-info {
            background: rgba(255,255,255,0.02);
            border-bottom: 1px solid rgba(255,255,255,0.05);
        }

        /* Folder header dark mode variants */
        .folder-header.folder-files {
            background: #374151;
            color: #e5e7eb;
        }

        .folder-header.folder-media {
            background: rgba(76, 175, 80, 0.25);
            color: #a5d6a7;
        }

        .folder-header.folder-documents {
            background: rgba(255, 152, 0, 0.25);
            color: #ffcc80;
        }

        .folder-header.folder-trash {
            background: rgba(244, 67, 54, 0.25);
            color: #ef9a9a;
        }

        /* File type dark mode variants */
        .sortable-item > .file-item.file-type-image {
            background: rgba(33, 150, 243, 0.25);
            color: #90caf9;
        }

        .sortable-item > .file-item.file-type-document {
            background: rgba(255, 152, 0, 0.25);
            color: #ffcc80;
        }

        .sortable-item > .file-item.file-type-video {
            background: rgba(156, 39, 176, 0.25);
            color: #ce93d8;
        }
    }

    .empty-state {
        color: var(--dxdnd-text-muted);
        text-align: center;
        padding: 40px 20px;
        font-style: italic;
        font-size: 14px;
    }

    /* Visual feedback for drop zones */
    .drop-zone.over.can-drop .folder-contents {
        background: rgba(76, 175, 80, 0.1);
        border: 2px dashed #4caf50;
        border-radius: 4px;
    }

    .drop-zone.over.cannot-drop .folder-contents {
        background: rgba(244, 67, 54, 0.05);
        border: 2px dashed #f44336;
        border-radius: 4px;
    }
"#;
