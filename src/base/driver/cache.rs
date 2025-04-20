use std::sync::Arc;
use std::path::PathBuf;
use std::fmt::Debug;

#[cfg(not(target_arch = "wasm32"))]
use tokio::sync::Mutex;

use super::state::Field;

#[cfg(target_os = "ios")]
extern "C" {
    fn get_application_support_dir() -> *const std::os::raw::c_char;
}

pub struct AppStorage;
impl AppStorage {
    fn _get_path(name: &str) -> PathBuf {
        #[cfg(target_os="linux")]
        {
            PathBuf::from(env!("HOME")).join(format!(".{name}"))
        }

        #[cfg(target_os="macos")]
        {
            PathBuf::from(env!("HOME")).join(format!(".{name}"))
        }

       #[cfg(target_os="ios")]
        unsafe {
            let ptr = get_application_support_dir();
            if ptr.is_null() {panic!("COULD NOT GET APPLICATION DIRECTORY");}
            let c_str = std::ffi::CStr::from_ptr(ptr);
            PathBuf::from(std::path::Path::new(&c_str.to_string_lossy().to_string()))
        }
    }

    pub fn get_path(name: &str) -> PathBuf {
        let path = Self::_get_path(name);
        // #[cfg(not(any(target_os="linux", target_os="ios")))] { unimplemented!(); }
        std::fs::create_dir_all(&*path).unwrap();
        path
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Clone)]
pub struct Cache(
    Arc<Mutex<rusqlite::Connection>>
);

#[cfg(not(target_arch = "wasm32"))]
impl Cache {
    pub(crate) async fn new(name: &str) -> Self {
        let path = AppStorage::get_path(name).join("cache.db");
        let db = rusqlite::Connection::open(path).unwrap();
        db.execute(
            "CREATE TABLE if not exists kvs(key TEXT NOT NULL UNIQUE, value TEXT);", []
        ).unwrap();
        Cache(Arc::new(Mutex::new(db)))
    }

    pub async fn set<F: Field + 'static>(&self, item: F) {
        self.0.lock().await.execute(
            "INSERT INTO kvs(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value;",
            [F::ident(), hex::encode(item.to_bytes())]
        ).unwrap();
    }
    pub async fn get<F: Field + 'static>(&self) -> F {
        let db = self.0.lock().await;
        let mut stmt = db.prepare(&format!(
            "SELECT value FROM kvs where key = \'{}\'",
            F::ident()
        )).unwrap();
        let result = stmt.query_and_then([], |row| {
            let item: String = row.get(0).unwrap();
            Ok(hex::decode(item).unwrap())
        }).unwrap().collect::<Result<Vec<Vec<u8>>, rusqlite::Error>>().unwrap();
        result.first().map(|b| F::from_bytes(b)).unwrap_or_default()
    }
}

//TODO: WASM Cache
