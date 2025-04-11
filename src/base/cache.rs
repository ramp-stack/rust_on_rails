use std::sync::{LazyLock, Mutex, Arc};
use std::path::PathBuf;
use std::fmt::Debug;

use super::Field;

#[cfg(target_os = "ios")]
extern "C" {
    fn get_application_support_dir() -> *const std::os::raw::c_char;
}

static APP_STORAGE_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
   #[cfg(target_os="linux")]
    {
        PathBuf::from(env!("HOME"))
    }

   #[cfg(target_os="ios")]
    unsafe {
        let ptr = get_application_support_dir();
        if ptr.is_null() {panic!("COULD NOT GET APPLICATION DIRECTORY");}
        let c_str = std::ffi::CStr::from_ptr(ptr);
        PathBuf::from(c_str.to_string_lossy())
    }

   #[cfg(not(any(target_os="linux", target_os="ios")))] { unimplemented!(); }
});

#[derive(Debug)]
pub struct Cache(
    #[cfg(not(target_arch = "wasm32"))]
    Arc<Mutex<rusqlite::Connection>>
);

impl Cache {
    pub fn new(name: &str) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::get_path(name).join("cache.db");
            let db = rusqlite::Connection::open(path).unwrap();
            db.execute(
                "CREATE TABLE if not exists kvs(key TEXT NOT NULL UNIQUE, value TEXT);", []
            ).unwrap();
            Cache(Arc::new(Mutex::new(db)))
        }
    }

    pub async fn set<F: Field + 'static>(&self, item: F) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.0.lock().unwrap().execute(
                "INSERT INTO kvs(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value;",
                [F::ident(), hex::encode(item.to_bytes())]
            ).unwrap();
        }
    }
    pub async fn get<F: Field + 'static>(&self) -> F {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let db = self.0.lock().unwrap();
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

    fn get_path(name: &str) -> PathBuf {
        let mut path = APP_STORAGE_DIR.clone();
        #[cfg(target_os="linux")] { path = path.join(".".to_string()+name); }
        std::fs::create_dir_all(&*path).unwrap();
        path
    }
}

//TODO: WASM Cache
