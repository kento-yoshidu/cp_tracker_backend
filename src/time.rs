pub fn now_jst() -> chrono::DateTime<chrono::FixedOffset> {
    let jst = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    chrono::Utc::now().with_timezone(&jst)
}
