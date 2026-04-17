//! 훅 기반 알림 인프라 (v0.6 Slice B).
//!
//! 외부 훅 스크립트가 `<vault>/.claude/state/notifications.jsonl`에 append한 라인을
//! watcher가 증분으로 읽어 `emit("notification", ...)`로 발화하고, 프론트의 Toast가 렌더한다.
//!
//! 앱은 이 파일을 **쓰지 않는다** (읽기 전용).

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};

/// 알림 레벨. jsonl의 `level` 필드는 이 4값 중 하나만 허용된다.
/// 그 외 값은 파싱 단계에서 해당 라인만 스킵된다.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Warn,
    Error,
    Success,
}

/// 단일 알림 이벤트. jsonl 1라인 또는 Tauri emit payload.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationEvent {
    pub level: NotificationLevel,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ts: Option<String>,
}

/// 볼트 루트 기준 notifications.jsonl 절대 경로.
pub fn notifications_path(vault_root: &Path) -> PathBuf {
    vault_root
        .join(".claude")
        .join("state")
        .join("notifications.jsonl")
}

/// 증분 읽기를 위한 오프셋. 이미 emit한 라인은 다시 emit하지 않도록 line_count를 기억.
#[derive(Debug, Default)]
pub struct NotificationsOffset {
    pub line_count: usize,
}

impl NotificationsOffset {
    /// 현재 notifications.jsonl의 라인 수로 초기화 (백로그 플러시 방지).
    /// 파일이 없거나 읽기 실패하면 0.
    pub fn from_current(vault_root: &Path) -> Self {
        let path = notifications_path(vault_root);
        let line_count = fs::read_to_string(&path)
            .map(|s| s.lines().count())
            .unwrap_or(0);
        Self { line_count }
    }

    /// 볼트 전환 시 새 볼트 기준으로 오프셋 재설정 (이전 볼트 오프셋 누수 방지).
    pub fn reset(&mut self, vault_root: &Path) {
        *self = Self::from_current(vault_root);
    }
}

/// 런타임에서 공유되는 오프셋 상태 (볼트 전환 시 reset).
pub type NotificationsState = Arc<Mutex<NotificationsOffset>>;

/// notifications.jsonl을 읽어 새 이벤트만 수집하고 오프셋을 갱신한다.
///
/// - 파일 미존재 / 읽기 실패 → 빈 Vec, 오프셋 변경 없음 (graceful)
/// - Mutex poison → 빈 Vec (호출자가 재시도 가능)
pub fn collect_new_events(
    vault_root: &Path,
    state: &NotificationsState,
) -> Vec<NotificationEvent> {
    let path = notifications_path(vault_root);
    let content = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let mut guard = match state.lock() {
        Ok(g) => g,
        Err(_) => return Vec::new(),
    };

    let (events, new_count) = read_new_notifications(&content, guard.line_count);
    guard.line_count = new_count;
    events
}

/// jsonl 문자열 전체를 파싱해 유효 이벤트만 반환.
/// - 빈/공백 라인 스킵
/// - JSON 파싱 실패 스킵 (깨진 라인 1개가 나머지를 망치지 않음)
/// - 알 수 없는 level 값 스킵 (serde enum deserialize가 에러)
/// - message가 빈 문자열이면 스킵
/// - 입력 순서 보존
///
/// 런타임에서는 `read_new_notifications(content, 0)`으로 동일 결과를 얻을 수 있어
/// 현재 내부 호출자가 없다. Spec에 명시된 공개 계약을 유지하며 향후 bulk import /
/// 디버깅용으로 직접 호출할 수 있도록 남긴다.
#[allow(dead_code)]
pub fn parse_notifications_jsonl(content: &str) -> Vec<NotificationEvent> {
    content
        .lines()
        .filter_map(parse_notification_line)
        .collect()
}

/// 라인 수 기반 증분 파서.
///
/// `prev_line_count` 이후에 추가된 라인만 파싱해 돌려준다.
/// 파일이 줄어든 경우(truncate/rotate) 전체를 다시 처리하고 새 line_count를 반환한다.
///
/// 반환: `(new_events, new_line_count)`. new_line_count는 **원본 라인 수** —
/// 스킵된(깨진) 라인도 카운트에 포함해 다음 호출에서 같은 라인을 재파싱하지 않는다.
pub fn read_new_notifications(
    content: &str,
    prev_line_count: usize,
) -> (Vec<NotificationEvent>, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let current_count = lines.len();

    let new_slice: &[&str] = if current_count < prev_line_count {
        // 축소(truncate/rotate) → 전체 재처리
        &lines[..]
    } else {
        &lines[prev_line_count..]
    };

    let events: Vec<NotificationEvent> = new_slice
        .iter()
        .filter_map(|l| parse_notification_line(l))
        .collect();

    (events, current_count)
}

/// 한 라인 → `Option<NotificationEvent>` (스킵 규약 1곳에 모음).
fn parse_notification_line(line: &str) -> Option<NotificationEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let ev: NotificationEvent = serde_json::from_str(trimmed).ok()?;
    if ev.message.is_empty() {
        return None;
    }
    Some(ev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // BC #18: notifications_path 은 <vault>/.claude/state/notifications.jsonl
    #[test]
    fn bc18_notifications_path_follows_convention() {
        let p = notifications_path(Path::new("/vault"));
        assert_eq!(p, PathBuf::from("/vault/.claude/state/notifications.jsonl"));
    }

    // level 직렬화가 lowercase 규약을 따르는지
    #[test]
    fn level_serializes_as_lowercase() {
        assert_eq!(
            serde_json::to_string(&NotificationLevel::Info).unwrap(),
            "\"info\""
        );
        assert_eq!(
            serde_json::to_string(&NotificationLevel::Warn).unwrap(),
            "\"warn\""
        );
        assert_eq!(
            serde_json::to_string(&NotificationLevel::Error).unwrap(),
            "\"error\""
        );
        assert_eq!(
            serde_json::to_string(&NotificationLevel::Success).unwrap(),
            "\"success\""
        );
    }

    // level 역직렬화 — 알 수 없는 값은 에러
    #[test]
    fn level_deserializes_known_values() {
        let info: NotificationLevel = serde_json::from_str("\"info\"").unwrap();
        assert_eq!(info, NotificationLevel::Info);

        let bad: Result<NotificationLevel, _> = serde_json::from_str("\"danger\"");
        assert!(bad.is_err());
    }

    // NotificationEvent 역직렬화: source/ts 없음 → None
    #[test]
    fn event_deserializes_without_optional_fields() {
        let ev: NotificationEvent =
            serde_json::from_str(r#"{"level":"info","message":"hi"}"#).unwrap();
        assert_eq!(ev.level, NotificationLevel::Info);
        assert_eq!(ev.message, "hi");
        assert!(ev.source.is_none());
        assert!(ev.ts.is_none());
    }

    // NotificationEvent 직렬화: None 필드는 생략
    #[test]
    fn event_serializes_without_optional_fields() {
        let ev = NotificationEvent {
            level: NotificationLevel::Success,
            message: "done".into(),
            source: None,
            ts: None,
        };
        let json = serde_json::to_string(&ev).unwrap();
        assert_eq!(json, r#"{"level":"success","message":"done"}"#);
    }

    // NotificationsOffset은 line_count = 0으로 기본 생성
    #[test]
    fn offset_default_starts_at_zero() {
        let off = NotificationsOffset::default();
        assert_eq!(off.line_count, 0);
    }

    // ----- parse_notifications_jsonl (BC #1~11) -----

    // BC #1
    #[test]
    fn bc1_empty_string_yields_empty_vec() {
        assert!(parse_notifications_jsonl("").is_empty());
    }

    // BC #2
    #[test]
    fn bc2_single_valid_line() {
        let events = parse_notifications_jsonl(r#"{"level":"info","message":"hi"}"#);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, NotificationLevel::Info);
        assert_eq!(events[0].message, "hi");
    }

    // BC #3
    #[test]
    fn bc3_three_valid_lines_preserve_order() {
        let jsonl = [
            r#"{"level":"info","message":"first"}"#,
            r#"{"level":"warn","message":"second"}"#,
            r#"{"level":"error","message":"third"}"#,
        ]
        .join("\n");
        let events = parse_notifications_jsonl(&jsonl);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].message, "first");
        assert_eq!(events[1].message, "second");
        assert_eq!(events[2].message, "third");
        assert_eq!(events[1].level, NotificationLevel::Warn);
    }

    // BC #4
    #[test]
    fn bc4_broken_json_line_is_skipped() {
        let jsonl = [
            r#"{"level":"info","message":"first"}"#,
            r#"not valid json"#,
            r#"{"level":"error","message":"third"}"#,
        ]
        .join("\n");
        let events = parse_notifications_jsonl(&jsonl);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "first");
        assert_eq!(events[1].message, "third");
    }

    // BC #5
    #[test]
    fn bc5_blank_and_whitespace_lines_are_skipped() {
        let jsonl = [
            r#"{"level":"info","message":"first"}"#,
            "",
            "   ",
            "\t",
            r#"{"level":"success","message":"second"}"#,
        ]
        .join("\n");
        let events = parse_notifications_jsonl(&jsonl);
        assert_eq!(events.len(), 2);
    }

    // BC #6: 알 수 없는 level → 스킵
    #[test]
    fn bc6_unknown_level_is_skipped() {
        let events = parse_notifications_jsonl(r#"{"level":"danger","message":"x"}"#);
        assert!(events.is_empty());
    }

    // BC #7: message 필드 누락 → 스킵
    #[test]
    fn bc7_missing_message_is_skipped() {
        let events = parse_notifications_jsonl(r#"{"level":"info"}"#);
        assert!(events.is_empty());
    }

    // BC #8: 빈 message → 스킵
    #[test]
    fn bc8_empty_message_is_skipped() {
        let events = parse_notifications_jsonl(r#"{"level":"info","message":""}"#);
        assert!(events.is_empty());
    }

    // BC #9: source/ts 옵션 필드 없어도 OK
    #[test]
    fn bc9_optional_fields_absent_is_ok() {
        let events = parse_notifications_jsonl(r#"{"level":"info","message":"x"}"#);
        assert_eq!(events.len(), 1);
        assert!(events[0].source.is_none());
        assert!(events[0].ts.is_none());
    }

    // BC #10: 모든 필드 채워짐
    #[test]
    fn bc10_all_fields_present() {
        let line = r#"{"level":"success","message":"saved","source":"scaffold","ts":"2026-04-17T12:34:56Z"}"#;
        let events = parse_notifications_jsonl(line);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, NotificationLevel::Success);
        assert_eq!(events[0].source.as_deref(), Some("scaffold"));
        assert_eq!(events[0].ts.as_deref(), Some("2026-04-17T12:34:56Z"));
    }

    // BC #11: trailing newline은 영향 없음
    #[test]
    fn bc11_trailing_newline_is_ignored() {
        let events = parse_notifications_jsonl("{\"level\":\"info\",\"message\":\"x\"}\n");
        assert_eq!(events.len(), 1);

        // 여러 trailing newline
        let events =
            parse_notifications_jsonl("{\"level\":\"info\",\"message\":\"x\"}\n\n\n");
        assert_eq!(events.len(), 1);
    }

    // ----- read_new_notifications (BC #12~17) -----

    fn make_jsonl(messages: &[&str]) -> String {
        messages
            .iter()
            .map(|m| format!(r#"{{"level":"info","message":"{m}"}}"#))
            .collect::<Vec<_>>()
            .join("\n")
    }

    // BC #12: content 5라인, prev == 5 → ([], 5)
    #[test]
    fn bc12_no_new_lines_yields_empty() {
        let content = make_jsonl(&["a", "b", "c", "d", "e"]);
        let (events, count) = read_new_notifications(&content, 5);
        assert!(events.is_empty());
        assert_eq!(count, 5);
    }

    // BC #13: content 5라인, prev == 3 → 마지막 2라인만 파싱
    #[test]
    fn bc13_appended_lines_are_parsed() {
        let content = make_jsonl(&["a", "b", "c", "d", "e"]);
        let (events, count) = read_new_notifications(&content, 3);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "d");
        assert_eq!(events[1].message, "e");
        assert_eq!(count, 5);
    }

    // BC #14: content 3라인, prev == 5 (축소) → 전체 재파싱, count == 3
    #[test]
    fn bc14_truncated_file_reparses_from_start() {
        let content = make_jsonl(&["x", "y", "z"]);
        let (events, count) = read_new_notifications(&content, 5);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].message, "x");
        assert_eq!(count, 3);
    }

    // BC #15: 빈 문자열 + prev 0 → ([], 0)
    #[test]
    fn bc15_empty_content_and_zero_prev() {
        let (events, count) = read_new_notifications("", 0);
        assert!(events.is_empty());
        assert_eq!(count, 0);
    }

    // BC #16: 5라인 중 1라인 깨짐, prev == 0 → 유효 4개, count == 5
    #[test]
    fn bc16_broken_line_counts_toward_line_count() {
        let content = [
            r#"{"level":"info","message":"a"}"#,
            r#"{"level":"info","message":"b"}"#,
            r#"not json"#,
            r#"{"level":"info","message":"d"}"#,
            r#"{"level":"info","message":"e"}"#,
        ]
        .join("\n");
        let (events, count) = read_new_notifications(&content, 0);
        assert_eq!(events.len(), 4);
        assert_eq!(count, 5); // 깨진 라인도 카운트
    }

    // BC #17: prev==3, 새 2라인 중 1라인 깨짐 → 유효 1개, count == 5
    #[test]
    fn bc17_partial_increment_with_broken_line() {
        let content = [
            r#"{"level":"info","message":"old1"}"#,
            r#"{"level":"info","message":"old2"}"#,
            r#"{"level":"info","message":"old3"}"#,
            r#"not json"#,
            r#"{"level":"info","message":"new"}"#,
        ]
        .join("\n");
        let (events, count) = read_new_notifications(&content, 3);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "new");
        assert_eq!(count, 5);
    }

    // 깨진 라인이 이미 처리된 구간에 있을 때 재파싱되지 않음
    #[test]
    fn broken_line_in_processed_range_is_not_reparsed() {
        let content = [
            r#"{"level":"info","message":"a"}"#,
            r#"not json"#, // 이미 스킵된 걸로 치고 처음에 line_count=2를 기록했다고 가정
            r#"{"level":"info","message":"c"}"#,
        ]
        .join("\n");
        // prev=2 → 세 번째 라인만 본다
        let (events, count) = read_new_notifications(&content, 2);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "c");
        assert_eq!(count, 3);
    }

    // ----- collect_new_events + NotificationsOffset I/O (BC #19~23) -----

    /// TempDir 기반 테스트 볼트 + 빈 notifications.jsonl 세팅 (선택).
    fn make_vault_with_notifications(content: Option<&str>) -> TempDir {
        let dir = TempDir::new().unwrap();
        if let Some(c) = content {
            let p = notifications_path(dir.path());
            fs::create_dir_all(p.parent().unwrap()).unwrap();
            fs::write(&p, c).unwrap();
        }
        dir
    }

    fn append_line(vault_root: &Path, line: &str) {
        let p = notifications_path(vault_root);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        let mut existing = fs::read_to_string(&p).unwrap_or_default();
        if !existing.is_empty() && !existing.ends_with('\n') {
            existing.push('\n');
        }
        existing.push_str(line);
        existing.push('\n');
        fs::write(&p, existing).unwrap();
    }

    fn state_from_offset(offset: NotificationsOffset) -> NotificationsState {
        Arc::new(Mutex::new(offset))
    }

    // BC #19: notifications.jsonl 미존재 → 빈 Vec, state 변경 없음
    #[test]
    fn bc19_missing_file_yields_empty_and_preserves_state() {
        let dir = TempDir::new().unwrap();
        let state = state_from_offset(NotificationsOffset { line_count: 0 });

        let events = collect_new_events(dir.path(), &state);
        assert!(events.is_empty());
        assert_eq!(state.lock().unwrap().line_count, 0);
    }

    // BC #20: 초기 기동 직후 (state = current line count) + 변경 없음 → 빈 Vec
    #[test]
    fn bc20_initialized_from_current_sees_no_backlog() {
        let existing = [
            r#"{"level":"info","message":"old1"}"#,
            r#"{"level":"info","message":"old2"}"#,
        ]
        .join("\n");
        let dir = make_vault_with_notifications(Some(&existing));

        let offset = NotificationsOffset::from_current(dir.path());
        assert_eq!(offset.line_count, 2);

        let state = state_from_offset(offset);
        let events = collect_new_events(dir.path(), &state);
        assert!(events.is_empty());
        assert_eq!(state.lock().unwrap().line_count, 2);
    }

    // BC #21: 2라인 append (유효) → 2개 이벤트, line_count += 2
    #[test]
    fn bc21_appended_valid_lines_yield_events() {
        let dir = make_vault_with_notifications(Some(""));
        let state = state_from_offset(NotificationsOffset::from_current(dir.path()));

        append_line(dir.path(), r#"{"level":"info","message":"a"}"#);
        append_line(dir.path(), r#"{"level":"success","message":"b"}"#);

        let events = collect_new_events(dir.path(), &state);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "a");
        assert_eq!(events[1].message, "b");
        assert_eq!(state.lock().unwrap().line_count, 2);
    }

    // BC #22: append된 라인 중 1개 깨짐 → 유효 건만, line_count는 원본 라인 수
    #[test]
    fn bc22_broken_line_still_advances_line_count() {
        let dir = make_vault_with_notifications(Some(""));
        let state = state_from_offset(NotificationsOffset::from_current(dir.path()));

        append_line(dir.path(), r#"{"level":"info","message":"a"}"#);
        append_line(dir.path(), r#"not json"#);
        append_line(dir.path(), r#"{"level":"info","message":"c"}"#);

        let events = collect_new_events(dir.path(), &state);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "a");
        assert_eq!(events[1].message, "c");
        assert_eq!(state.lock().unwrap().line_count, 3);
    }

    // BC #23: truncate 후 1라인 append → 전체 재처리, line_count == 1
    #[test]
    fn bc23_truncate_then_append_reparses_everything() {
        let initial = [
            r#"{"level":"info","message":"old1"}"#,
            r#"{"level":"info","message":"old2"}"#,
            r#"{"level":"info","message":"old3"}"#,
        ]
        .join("\n");
        let dir = make_vault_with_notifications(Some(&initial));
        let state = state_from_offset(NotificationsOffset::from_current(dir.path()));
        assert_eq!(state.lock().unwrap().line_count, 3);

        // 파일을 1라인으로 축소
        let p = notifications_path(dir.path());
        fs::write(&p, "{\"level\":\"warn\",\"message\":\"fresh\"}\n").unwrap();

        let events = collect_new_events(dir.path(), &state);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].message, "fresh");
        assert_eq!(events[0].level, NotificationLevel::Warn);
        assert_eq!(state.lock().unwrap().line_count, 1);
    }

    // reset: 새 볼트 기준으로 오프셋 교체
    #[test]
    fn reset_rebases_offset_on_new_vault() {
        let old_vault = make_vault_with_notifications(Some(
            &[
                r#"{"level":"info","message":"a"}"#,
                r#"{"level":"info","message":"b"}"#,
            ]
            .join("\n"),
        ));
        let new_vault = make_vault_with_notifications(Some(
            r#"{"level":"info","message":"only one"}"#,
        ));

        let mut offset = NotificationsOffset::from_current(old_vault.path());
        assert_eq!(offset.line_count, 2);

        offset.reset(new_vault.path());
        assert_eq!(offset.line_count, 1);
    }

    // 결합 시나리오: 모든 종류 섞임
    #[test]
    fn mixed_scenario_returns_only_valid_events() {
        let jsonl = [
            r#"{"level":"info","message":"one"}"#,
            "",
            r#"not json"#,
            r#"{"level":"danger","message":"two"}"#, // 알 수 없는 level
            r#"{"level":"warn"}"#,                   // message 없음
            r#"{"level":"success","message":"three"}"#,
        ]
        .join("\n");
        let events = parse_notifications_jsonl(&jsonl);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].message, "one");
        assert_eq!(events[1].message, "three");
    }
}
