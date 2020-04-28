use crate::lmdb_web_site;
use std::{
    collections::HashMap,
    io,
    fs,
    path,
};

pub struct Storage {
    path: path::PathBuf,
    dbs: HashMap<String, lmdb_web_site::LMDBWebSite>,
}

impl Storage {
    pub fn new(path: path::PathBuf) -> Self {
        Storage {
            path: path,
            dbs: HashMap::new(),
        }
    }

    pub fn get(&mut self, name: &str) -> Result<lmdb_web_site::LMDBWebSite, lmdb::Error> {
        match self.dbs.get(name) {
            Some(db) => Ok(db.clone()),
            None => {
                let mut path = self.path.clone();
                path.push(path::Path::new(name));

                // Make an effort to create the dir if needed. If this fails,
                // it may be because it already exists, and if it's a "real"
                // failure we'll hit it later anyway, so ignore the result:
                let _ = fs::create_dir_all(&path);

                let lmdb_site = lmdb_web_site::LMDBWebSite::open(
                    String::from("site"),
                    String::from("http://example.com"),
                    &path,
                )?;
                self.dbs.insert(String::from(name), lmdb_site.clone());
                Ok(lmdb_site)
            }
        }
    }

    pub fn list_sites(&self) -> io::Result<Vec<String>> {
        fs::read_dir(&self.path)?.map(|r| r.map(|item| {
            item.path()
                .file_name().unwrap()
                .to_os_string()
                .into_string().unwrap()
        })).collect()
    }
}
