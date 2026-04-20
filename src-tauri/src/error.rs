use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("파일 읽기 실패: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML 파싱 실패: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("볼트 경로를 찾을 수 없음: {0}")]
    VaultNotFound(String),

    #[error("볼트가 연결되어 있지 않음")]
    VaultNotConfigured,

    #[error("선택한 경로에 기존 파일/폴더가 있습니다: {0}")]
    VaultDirectoryNotEmpty(String),

    #[error("노트를 찾을 수 없음: {0}")]
    NoteNotFound(String),

    #[error("네트워크 요청 실패: {0}")]
    Network(String),

    #[allow(dead_code)]
    #[error("HTML 파싱 실패: {0}")]
    HtmlParse(String),

    #[error("검색 오류: {0}")]
    Search(String),

    #[error("유효하지 않은 파일 경로: {0}")]
    InvalidPath(String),

    #[error("지원하지 않는 확장자: {0}")]
    InvalidExtension(String),

    #[error("녹음 데이터가 비어 있음")]
    EmptyRecording,

    #[error("파일이 이미 존재합니다: {0}")]
    FileExists(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
