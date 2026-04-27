use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchKind {
    File,
    Node,
    Qa,
}

impl SearchKind {
    pub fn as_index_str(&self) -> &'static str {
        match self {
            SearchKind::File => "file",
            SearchKind::Node => "node",
            SearchKind::Qa => "qa",
        }
    }

    pub fn from_index_str(s: &str) -> Option<Self> {
        match s {
            "file" => Some(SearchKind::File),
            "node" => Some(SearchKind::Node),
            "qa" => Some(SearchKind::Qa),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchHit {
    pub project_id: String,
    pub project_name: String,
    pub kind: SearchKind,
    pub title: String,
    pub snippet: String,
    pub path: String,
    pub score: f32,
}
