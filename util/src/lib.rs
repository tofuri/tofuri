pub fn timestamp() -> u32 {
    chrono::offset::Utc::now().timestamp() as u32
}
