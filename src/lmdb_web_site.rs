use crate::shortcuts::entity_list;
use lmdb;
use lmdb::Transaction;
use std::{
    path,
    rc::Rc,
};
use capnp::{Error, capability::Promise};
use sandstorm::{
    util_capnp::assignable,
    web_publishing_capnp::web_site,
};

#[derive(Clone, Debug)]
pub struct LMDBWebSite {
    db_name: String,
    url: String,
    env: Rc<lmdb::Environment>,
    db: Rc<lmdb::Database>,
}

#[derive(Clone, Debug)]
struct EntitiesCell(Rc<LMDBWebSite>);

pub fn db_err(_: lmdb::Error) -> Error {
    Error::failed(String::from("Database Error"))
}

impl LMDBWebSite {
    pub fn open(db_name: String, url: String, p: &path::Path) -> lmdb::Result<Self> {
        let env = lmdb::Environment::new().open(p)?;
        let db = env.open_db(Some(&db_name[..])).or_else(|_| {
            env.create_db(None, lmdb::DatabaseFlags::empty())
        })?;
        Ok(LMDBWebSite {
            db_name: db_name,
            url: url,
            db: Rc::new(db),
            env: Rc::new(env),
        })
    }
}

impl web_site::Server for LMDBWebSite {
    fn get_url(&mut self,
               _params: web_site::GetUrlParams,
               mut results: web_site::GetUrlResults) -> Promise<(), Error> {
        results.get().set_path(&self.url);
        Promise::ok(())
    }

    fn get_subsite(&mut self,
                   params: web_site::GetSubsiteParams,
                   mut results: web_site::GetSubsiteResults) -> Promise<(), Error> {
        let mut site = self.clone();
        Promise::from_future(async move {
            site.url += params.get()?.get_prefix()?;
            results.get().set_site(capnp_rpc::new_client(site));
            Ok(())
        })
    }

    fn get_entities(&mut self,
                    params: web_site::GetEntitiesParams,
                    mut results: web_site::GetEntitiesResults) -> Promise<(), Error> {
        let mut site = self.clone();
        Promise::from_future(async move {
            site.url += params.get()?.get_path()?;
            results.get().set_entities(capnp_rpc::new_client(EntitiesCell(Rc::new(site))));
            Ok(())
        })
    }
}

impl assignable::Server<entity_list::Owned> for EntitiesCell {
    fn as_getter(&mut self,
                 _params: assignable::AsGetterParams<entity_list::Owned>,
                 mut results: assignable::AsGetterResults<entity_list::Owned>) -> Promise<(), Error> {
        results.get().set_getter(capnp_rpc::new_client(self.clone()));
        Promise::ok(())
    }

    fn as_setter(&mut self,
                 _params: assignable::AsSetterParams<entity_list::Owned>,
                 mut results: assignable::AsSetterResults<entity_list::Owned>) -> Promise<(), Error> {
        results.get().set_setter(capnp_rpc::new_client(self.clone()));
        Promise::ok(())
    }
}

impl assignable::setter::Server<entity_list::Owned> for EntitiesCell {
    fn set(&mut self,
           params: assignable::setter::SetParams<entity_list::Owned>,
           mut _results: assignable::setter::SetResults<entity_list::Owned>) -> Promise<(), Error> {
        let entities = self.clone();
        Promise::from_future(async move {
            let value = params.get()?.get_value()?;
            if value.len() == 0 {
                let site = &*entities.0;
                let env = &*site.env;
                let db = *site.db;
                let mut txn = env.begin_rw_txn().map_err(db_err)?;
                txn.del(db, &site.url, None).map_err(db_err)?;
                txn.commit().map_err(db_err)?
            } else {
                let mut msg = capnp::message::Builder::new_default();
                msg.set_root(value)?;
                let mut buffer = vec![];
                capnp::serialize::write_message(&mut buffer, &msg)?;

                let site = &*entities.0;
                let env = &*site.env;
                let db = *site.db;
                let mut txn = env.begin_rw_txn().map_err(db_err)?;
                txn.put(db, &site.url, &buffer, lmdb::WriteFlags::empty()).map_err(db_err)?;
                txn.commit().map_err(db_err)?
            }
            Ok(())
        })
    }
}

impl assignable::getter::Server<entity_list::Owned> for EntitiesCell {
    fn get(&mut self,
           _params: assignable::getter::GetParams<entity_list::Owned>,
           mut results: assignable::getter::GetResults<entity_list::Owned>) -> Promise<(), Error> {
        let entities = self.clone();
        Promise::from_future(async move {
            let site = &*entities.0;
            let env = &*site.env;
            let db = *site.db;
            let txn = env.begin_ro_txn().map_err(db_err)?;
            let mut bytes: &[u8] = match txn.get(db, &site.url) {
                Ok(res) => res,
                Err(lmdb::Error::NotFound) => {
                    // Just return null
                    return Ok(())
                },
                Err(e) => {
                    return Err(db_err(e))
                }
            };
            let msg =
                capnp::serialize::read_message_from_flat_slice(
                    &mut bytes,
                    Default::default(),
                )?;
            let src_list: entity_list::Reader = msg.get_root()?;
            results.get().set_value(src_list)?;
            Ok(())
        })
    }
}
