use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum NoteType {
    Til,
    Decision,
    Reading,
    Meeting,
    Idea,
    Artifact,
    Clipping,
    Moc,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum NoteStatus {
    Seedling,
    Growing,
    Evergreen,
    Stale,
    #[serde(other)]
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frontmatter {
    #[serde(rename = "type")]
    pub note_type: NoteType,
    pub created: NaiveDate,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub status: Option<NoteStatus>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_yaml_ng::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteEntry {
    pub path: String,
    pub title: String,
    pub frontmatter: Option<Frontmatter>,
    pub outgoing_links: Vec<String>,
    pub modified_at: i64,
    pub size: u64,
    #[serde(skip)]
    pub body: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub frontmatter: Option<Frontmatter>,
    pub modified_at: i64,
    pub snippet: Option<String>,
    pub match_field: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VaultIndex {
    pub notes: Vec<NoteEntry>,
    pub backlinks: HashMap<String, Vec<String>>,
    pub scanned_at: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultStats {
    pub total_notes: usize,
    pub by_type: HashMap<String, usize>,
    pub by_status: HashMap<String, usize>,
    pub by_folder: HashMap<String, usize>,
    pub total_links: usize,
    pub total_tags: usize,
    pub orphan_notes: usize,
    pub broken_links: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagInfo {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphNode {
    pub id: String,
    pub path: String,
    pub title: String,
    pub note_type: Option<NoteType>,
    pub link_count: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GodNode {
    pub path: String,
    pub title: String,
    pub note_type: Option<NoteType>,
    pub backlink_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderedNote {
    pub path: String,
    pub title: String,
    pub frontmatter: Option<Frontmatter>,
    pub html: String,
    pub outgoing_links: Vec<String>,
    pub backlinks: Vec<BacklinkEntry>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BacklinkEntry {
    pub path: String,
    pub title: String,
    pub note_type: Option<NoteType>,
    pub context: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct ClipRequest {
    pub url: String,
    pub tags: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize)]
pub struct ClipResult {
    pub path: String,
    pub title: String,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VaultChangeEvent {
    pub kind: ChangeKind,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderNode {
    pub name: String,
    pub path: String,
    pub note_count: usize,
    pub children: Vec<FolderNode>,
}
