use std::hash::{DefaultHasher, Hasher, Hash};
use std::path::PathBuf;
use std::any::TypeId;
use std::fmt::Debug;

use serde::{Serialize, Deserialize};

#[cfg(target_os = "ios")]
        if let Some(new_path) = get_app_support_path() {
            let path = new_path;
        };



#[cfg(target_os = "ios")]extern "C" {
    fn get_application_support_dir() -> *const std::os::raw::c_char;
}

#[cfg(target_os = "ios")]fn get_app_support_path() -> Option<String> {
    unsafe {
        let ptr = get_application_support_dir();
        if ptr.is_null() {
            println!("COULD NOT GET APPLICATION DIRECTORY");
            return None;
        }
        let c_str = std::ffi::CStr::from_ptr(ptr);
        Some(c_str.to_string_lossy().into_owned())
    }
}

pub trait Field: Serialize + for<'a> Deserialize <'a> + Default + Debug {
    fn ident() -> [u8; 8] where Self: Sized + 'static {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<Self>().hash(&mut hasher);
        let key = hasher.finish();
        key.to_le_bytes()
    }
    fn to_bytes(&self) -> Result<Vec<u8>, Error> {Ok(serde_json::to_vec(self).unwrap())}
    fn from_bytes(bytes: &[u8]) -> Result<Self, Error> where Self: Sized {
        Ok(serde_json::from_slice(bytes).unwrap())
    }
}

impl<I: Serialize + for<'a> Deserialize <'a> + Default + Debug> Field for I {}


#[cfg(not(target_arch = "wasm32"))]
use rusqlite::{Error, Connection};

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct State(Connection);

#[cfg(not(target_arch = "wasm32"))]
impl State {
    pub fn new(path: PathBuf) -> Result<Self, Error> {
        // std::fs::create_dir_all(path.clone()).unwrap();
        let db = Connection::open(path.join("kvs.db"))?;
        db.execute("CREATE TABLE if not exists kvs(key TEXT NOT NULL UNIQUE, value TEXT);", [])?;
        Ok(State(db))
    }

    fn inner_get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Error> {
        let mut stmt = self.0.prepare(&format!("SELECT value FROM kvs where key = \'{}\'", hex::encode(key)))?;
        let result = stmt.query_and_then([], |row| {
            let item: String = row.get(0)?;
            Ok(hex::decode(item).unwrap())
        })?.collect::<Result<Vec<Vec<u8>>, Error>>()?;
        Ok(result.first().cloned())
    }

    fn inner_set(&self, key: &[u8], value: &[u8]) -> Result<(), Error> {
        self.0.execute("
            INSERT INTO kvs(key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value;
        ", [hex::encode(key), hex::encode(value)])?;
        Ok(())
    }

    pub fn set<F: Field + 'static>(&self, item: &F) -> Result<(), Error> {
        self.inner_set(&F::ident(), &item.to_bytes()?)
    }
    pub fn get<F: Field + 'static>(&self) -> Result<F, Error> {
        Ok(self.inner_get(&F::ident())?.map(|b| F::from_bytes(&b)).transpose()?.unwrap_or_default())
    }
}
