/// HTML → Markdown 변환
pub fn html_to_markdown(html: &str) -> String {
    let md = html2md::parse_html(html);
    // 과도한 빈 줄 정리
    let mut result = String::new();
    let mut blank_count = 0;
    for line in md.lines() {
        if line.trim().is_empty() {
            blank_count += 1;
            if blank_count <= 2 {
                result.push('\n');
            }
        } else {
            blank_count = 0;
            result.push_str(line);
            result.push('\n');
        }
    }
    result.trim().to_string()
}
