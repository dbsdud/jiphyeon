use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

use tauri::State;

use crate::config::ConfigState;
use crate::error::AppError;
use crate::models::{
    ClusterSummary, FolderNode, GodNode, GraphEdge, GraphNode, LinkGraph, NoteEntry, SearchResult,
    TagInfo, VaultIndex, VaultStats,
};
use crate::vault::{indexer, search};

pub type VaultState = Arc<RwLock<VaultIndex>>;

#[tauri::command]
pub fn get_vault_stats(state: State<'_, VaultState>) -> Result<VaultStats, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(indexer::compute_stats(&index))
}

#[tauri::command]
pub fn get_orphan_notes(state: State<'_, VaultState>) -> Result<Vec<NoteEntry>, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(indexer::find_orphan_notes(&index))
}

#[tauri::command]
pub fn get_top_god_nodes(
    state: State<'_, VaultState>,
    limit: usize,
) -> Result<Vec<GodNode>, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(indexer::compute_top_god_nodes(&index, limit))
}

#[tauri::command]
pub fn get_cluster_summary(state: State<'_, VaultState>) -> Result<ClusterSummary, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    Ok(indexer::compute_clusters(&index))
}

#[tauri::command]
pub fn get_note_list(
    state: State<'_, VaultState>,
    folder: Option<String>,
    note_type: Option<String>,
    status: Option<String>,
    tag: Option<String>,
    query: Option<String>,
    sort_by: Option<String>,
) -> Result<Vec<NoteEntry>, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let mut notes: Vec<NoteEntry> = index
        .notes
        .iter()
        .filter(|n| {
            if let Some(ref f) = folder {
                let note_folder = Path::new(&n.path)
                    .parent()
                    .and_then(|p| p.to_str())
                    .unwrap_or("");
                if note_folder != f {
                    return false;
                }
            }
            if let Some(ref t) = note_type {
                match &n.frontmatter {
                    Some(fm) => {
                        let type_str = serde_json::to_string(&fm.note_type)
                            .unwrap_or_default()
                            .trim_matches('"')
                            .to_string();
                        if type_str != *t {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            if let Some(ref s) = status {
                match &n.frontmatter {
                    Some(fm) => match &fm.status {
                        Some(st) => {
                            let status_str = serde_json::to_string(st)
                                .unwrap_or_default()
                                .trim_matches('"')
                                .to_string();
                            if status_str != *s {
                                return false;
                            }
                        }
                        None => return false,
                    },
                    None => return false,
                }
            }
            if let Some(ref tg) = tag {
                match &n.frontmatter {
                    Some(fm) => {
                        if !fm.tags.contains(tg) {
                            return false;
                        }
                    }
                    None => return false,
                }
            }
            if let Some(ref q) = query {
                let q_lower = q.to_lowercase();
                let title_match = n.title.to_lowercase().contains(&q_lower);
                let tag_match = n
                    .frontmatter
                    .as_ref()
                    .map(|fm| fm.tags.iter().any(|t| t.to_lowercase().contains(&q_lower)))
                    .unwrap_or(false);
                if !title_match && !tag_match {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect();

    match sort_by.as_deref() {
        Some("title") => notes.sort_by(|a, b| a.title.cmp(&b.title)),
        Some("size") => notes.sort_by(|a, b| b.size.cmp(&a.size)),
        _ => notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at)), // 기본: 최근 수정순
    }

    Ok(notes)
}

#[tauri::command]
pub fn get_tag_list(state: State<'_, VaultState>) -> Result<Vec<TagInfo>, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let mut tag_counts: HashMap<String, usize> = HashMap::new();

    for note in &index.notes {
        if let Some(ref fm) = note.frontmatter {
            for tag in &fm.tags {
                *tag_counts.entry(tag.clone()).or_default() += 1;
            }
        }
    }

    let mut tags: Vec<TagInfo> = tag_counts
        .into_iter()
        .map(|(name, count)| TagInfo { name, count })
        .collect();
    tags.sort_by(|a, b| b.count.cmp(&a.count));

    Ok(tags)
}

#[tauri::command]
pub fn get_link_graph(state: State<'_, VaultState>) -> Result<LinkGraph, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;

    let nodes: Vec<GraphNode> = index
        .notes
        .iter()
        .map(|n| GraphNode {
            id: n.title.clone(),
            path: n.path.clone(),
            title: n.title.clone(),
            note_type: n.frontmatter.as_ref().map(|fm| fm.note_type.clone()),
            link_count: n.outgoing_links.len()
                + index.backlinks.get(&n.title).map_or(0, |bl| bl.len()),
        })
        .collect();

    let mut edges: Vec<GraphEdge> = Vec::new();
    for note in &index.notes {
        for link in &note.outgoing_links {
            edges.push(GraphEdge {
                source: note.title.clone(),
                target: link.clone(),
            });
        }
    }

    Ok(LinkGraph { nodes, edges })
}

#[tauri::command]
pub fn get_recent_notes(
    state: State<'_, VaultState>,
    config_state: State<'_, ConfigState>,
    limit: Option<usize>,
) -> Result<Vec<NoteEntry>, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let limit = limit.unwrap_or(config.recent_notes_limit);

    let mut notes = index.notes.clone();
    notes.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
    notes.truncate(limit);

    Ok(notes)
}

/// 폴더별 노트 수 맵에서 계층적 트리를 구축
fn build_tree(prefix: &str, counts: &HashMap<String, usize>) -> Vec<FolderNode> {
    let mut children_map: HashMap<String, usize> = HashMap::new();

    for (folder_path, &count) in counts {
        let relative = if prefix.is_empty() {
            folder_path.as_str()
        } else if let Some(rest) = folder_path.strip_prefix(prefix) {
            rest.strip_prefix('/').unwrap_or("")
        } else {
            continue;
        };

        if relative.is_empty() {
            continue;
        }

        let direct_child = match relative.find('/') {
            Some(slash) => &relative[..slash],
            None => relative,
        };

        // 직접 자식 폴더인 경우만 count 합산
        let child_path = if prefix.is_empty() {
            direct_child.to_string()
        } else {
            format!("{}/{}", prefix, direct_child)
        };

        if folder_path == &child_path {
            *children_map.entry(direct_child.to_string()).or_default() += count;
        } else {
            children_map.entry(direct_child.to_string()).or_default();
        }
    }

    let mut result: Vec<FolderNode> = children_map
        .into_iter()
        .map(|(name, direct_count)| {
            let full_path = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{}/{}", prefix, name)
            };
            let children = build_tree(&full_path, counts);
            FolderNode {
                name,
                path: full_path,
                note_count: direct_count,
                children,
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));

    // 루트 레벨에서만 "." 노드 추가
    if prefix.is_empty() {
        if let Some(&root_count) = counts.get("") {
            result.insert(
                0,
                FolderNode {
                    name: ".".to_string(),
                    path: "".to_string(),
                    note_count: root_count,
                    children: vec![],
                },
            );
        }
    }

    result
}

#[tauri::command]
pub fn get_folder_tree(state: State<'_, VaultState>) -> Result<Vec<FolderNode>, AppError> {
    let index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;

    let mut folder_counts: HashMap<String, usize> = HashMap::new();
    for note in &index.notes {
        let folder = Path::new(&note.path)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        *folder_counts.entry(folder).or_default() += 1;
    }

    Ok(build_tree("", &folder_counts))
}

#[tauri::command]
pub fn search_notes(
    state: State<'_, VaultState>,
    search_state: State<'_, search::SearchState>,
    query: String,
) -> Result<Vec<SearchResult>, AppError> {
    let vault_index = state.read().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let search_index = search_state
        .read()
        .map_err(|e| AppError::Search(e.to_string()))?;

    search::execute_search(&search_index, &vault_index, &query, 50)
}

#[tauri::command]
pub fn rescan_vault(
    state: State<'_, VaultState>,
    search_state: State<'_, search::SearchState>,
    config_state: State<'_, ConfigState>,
) -> Result<VaultStats, AppError> {
    let config = config_state
        .read()
        .map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    let vault_path = config
        .vault_path
        .as_ref()
        .ok_or(AppError::VaultNotConfigured)?;
    let new_index = indexer::scan_vault(vault_path, &config.exclude_dirs)?;
    let stats = indexer::compute_stats(&new_index);

    // 검색 인덱스 재구축
    let new_search = search::build_search_index(&new_index.notes)
        .map_err(|e| AppError::Search(e.to_string()))?;
    let mut si = search_state
        .write()
        .map_err(|e| AppError::Search(e.to_string()))?;
    *si = new_search;

    let mut index = state.write().map_err(|e| AppError::VaultNotFound(e.to_string()))?;
    *index = new_index;

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counts(entries: &[(&str, usize)]) -> HashMap<String, usize> {
        entries.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    #[test]
    fn build_tree_empty_vault() {
        let result = build_tree("", &HashMap::new());
        assert!(result.is_empty());
    }

    #[test]
    fn build_tree_root_only() {
        let c = counts(&[("", 3)]);
        let result = build_tree("", &c);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, ".");
        assert_eq!(result[0].note_count, 3);
        assert!(result[0].children.is_empty());
    }

    #[test]
    fn build_tree_single_level() {
        let c = counts(&[("dev", 2), ("meeting", 1)]);
        let result = build_tree("", &c);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, "dev");
        assert_eq!(result[0].note_count, 2);
        assert_eq!(result[1].name, "meeting");
        assert_eq!(result[1].note_count, 1);
    }

    #[test]
    fn build_tree_nested() {
        let c = counts(&[("dev", 1), ("dev/rust", 2)]);
        let result = build_tree("", &c);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "dev");
        assert_eq!(result[0].note_count, 1);
        assert_eq!(result[0].children.len(), 1);
        assert_eq!(result[0].children[0].name, "rust");
        assert_eq!(result[0].children[0].path, "dev/rust");
        assert_eq!(result[0].children[0].note_count, 2);
    }

    #[test]
    fn build_tree_three_levels() {
        let c = counts(&[("a", 1), ("a/b", 0), ("a/b/c", 3)]);
        let result = build_tree("", &c);
        assert_eq!(result.len(), 1);
        let a = &result[0];
        assert_eq!(a.name, "a");
        assert_eq!(a.note_count, 1);
        let b = &a.children[0];
        assert_eq!(b.name, "b");
        assert_eq!(b.note_count, 0);
        let cc = &b.children[0];
        assert_eq!(cc.name, "c");
        assert_eq!(cc.note_count, 3);
    }

    #[test]
    fn build_tree_sorted_alphabetically() {
        let c = counts(&[("zzz", 1), ("aaa", 1), ("mmm", 1)]);
        let result = build_tree("", &c);
        let names: Vec<&str> = result.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, vec!["aaa", "mmm", "zzz"]);
    }

    #[test]
    fn build_tree_root_plus_folders() {
        let c = counts(&[("", 2), ("dev", 3)]);
        let result = build_tree("", &c);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].name, ".");
        assert_eq!(result[0].note_count, 2);
        assert_eq!(result[1].name, "dev");
        assert_eq!(result[1].note_count, 3);
    }

    #[test]
    fn build_tree_parent_without_direct_notes() {
        // 폴더에 직접 노트 없고 하위에만 있는 경우
        let c = counts(&[("dev/rust", 5)]);
        let result = build_tree("", &c);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "dev");
        assert_eq!(result[0].note_count, 0);
        assert_eq!(result[0].children[0].name, "rust");
        assert_eq!(result[0].children[0].note_count, 5);
    }
}
