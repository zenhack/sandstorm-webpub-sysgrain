use lmdb;
use std::{
    path,
    rc::Rc,
};
use capnp::{Error, capability::Promise};
use sandstorm::{
    util_capnp::assignable,
    web_publishing_capnp::web_site,
};

#[derive(Clone)]
pub struct LMDBWebSite {
    db_name: String,
    url: String,
    env: Rc<lmdb::Environment>,
    db: Rc<lmdb::Database>,
}

#[derive(Clone)]
struct EntitiesCell(Rc<LMDBWebSite>);

impl LMDBWebSite {
    pub fn open(db_name: String, url: String, p: &path::Path) -> lmdb::Result<Self> {
        let env = lmdb::Environment::new().open(p)?;
        let db = env.open_db(Some(&db_name[..]))?;
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
            results.get().set_site(web_site::ToClient::new(site)
                                   .into_client::<capnp_rpc::Server>());
            Ok(())
        })
    }

    fn get_entities(&mut self,
                    params: web_site::GetEntitiesParams,
                    mut results: web_site::GetEntitiesResults) -> Promise<(), Error> {
        let mut site = self.clone();
        Promise::from_future(async move {
            site.url += params.get()?.get_path()?;
            results.get().set_entities(assignable::ToClient::new(EntitiesCell(Rc::new(site)))
                                       .into_client::<capnp_rpc::Server>());
            Ok(())
        })
    }
}

fn get_value<'a>(entities: &'a EntitiesCell) -> lmdb::Result<&'a capnp::struct_list::Reader<'a, web_site::entity::Owned>> {
    /*
    let env = &entities.get().env.get();
    let db = &entities.get().db.get();
    let txn = env.begin_ro_txn()?;
    let mut bytes: &[u8] = txn.get(db, key);
    capnp::serialize::read_message_from_flat_slice(
        &mut bytes,
        Default::default(),
    )?;
    */
    panic!("TODO")
}

fn set_value(entities: &mut EntitiesCell,
             value: &capnp::struct_list::Reader<web_site::entity::Owned>) -> lmdb::Result<()> {
    panic!("TODO")
}

fn delete_value(entities: &mut EntitiesCell) -> lmdb::Result<()> {
    panic!("TODO")
}

mod entity_list {
    use sandstorm::web_publishing_capnp::web_site;
    pub type Owned = capnp::struct_list::Owned<web_site::entity::Owned>;
}

impl assignable::Server<entity_list::Owned> for EntitiesCell {
    fn as_setter(&mut self,
                 params: assignable::AsSetterParams<entity_list::Owned>,
                 mut results: assignable::AsSetterResults<entity_list::Owned>) -> Promise<(), Error> {
        let ret = self.clone();
        results.get().set_setter(assignable::setter::ToClient::new(ret)
                                 .into_client::<capnp_rpc::Server>());
        Promise::ok(())
    }
}

impl assignable::setter::Server<entity_list::Owned> for EntitiesCell {
    fn set(&mut self,
           params: assignable::setter::SetParams<entity_list::Owned>,
           mut _results: assignable::setter::SetResults<entity_list::Owned>) -> Promise<(), Error> {
        let mut entities = self.clone();
        Promise::from_future(async move {
            let value = params.get()?.get_value()?;
            if value.len() == 0 {
                delete_value(&mut entities)
                    .map_err(|_| Error::failed(String::from("Database Error")))
            } else {
                set_value(&mut entities, &value)
                    .map_err(|_| Error::failed(String::from("Database Error")))
            }
        })
    }
}

impl assignable::getter::Server<entity_list::Owned> for EntitiesCell {
    fn get(&mut self,
           params: assignable::getter::GetParams<entity_list::Owned>,
           mut results: assignable::getter::GetResults<entity_list::Owned>) -> Promise<(), Error> {
        let entities = self.clone();
        Promise::from_future(async move {
            match get_value(&entities) {
                Ok(res) => {
                    //TODO: results.set_value(res);
                    Ok(())
                },
                Err(lmdb::Error::NotFound) => {
                    // Just return null
                    Ok(())
                },
                Err(_) => {
                    Err(Error::failed(String::from("Database Error")))
                }
            }
        })
    }
}
