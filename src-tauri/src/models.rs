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

#[allow(dead_code)]
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
