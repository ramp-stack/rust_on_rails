use simple_database::SqliteStore;

pub type Store = SqliteStore;
pub const STORAGE: std::path::Path = env!("CARGO_MANIFEST_PATH");
