use std::{
    env,
    path::PathBuf,
    sync::{Arc, Mutex},
    result,
};
use capnp::{traits::HasTypeId};
use sandstorm::{
    grain_capnp::{ui_view, ui_session},
    web_publishing_capnp::web_site,
    web_session_capnp::web_session,
};
use crate::{
    admin_ui,
    promise_util::{Promise, ok},
    lmdb_web_site,
    web_site_session,
    storage::Storage,
};

pub struct MainViewImpl {
    storage: Arc<Mutex<Storage>>,
}

impl MainViewImpl {
    pub fn new(site_dir: PathBuf) -> MainViewImpl {
        MainViewImpl{
            storage: Arc::new(Mutex::new(Storage::new(site_dir)))
        }
    }

    pub fn new_from_env() -> Result<MainViewImpl, env::VarError> {
        Ok(Self::new(PathBuf::from(env::var("WEB_SITES_DIR")?)))
    }

    fn get_site(&mut self, name: &str) -> result::Result<lmdb_web_site::LMDBWebSite, capnp::Error> {
        self.storage.lock().unwrap().get(name).map_err(lmdb_web_site::db_err)
    }
}

impl ui_view::Server for MainViewImpl {
    fn get_view_info(&mut self,
                     _params: ui_view::GetViewInfoParams,
                     mut results: ui_view::GetViewInfoResults) -> Promise {
        let mut mr = results.get().init_match_requests(1);
        let desc = mr.reborrow().get(0);
        let mut tags = desc.init_tags(1);
        let mut tag = tags.reborrow().get(0);
        tag.set_id(web_site::Client::type_id());
        ok()
    }

    fn new_session(&mut self,
                   _params: ui_view::NewSessionParams,
                   mut results: ui_view::NewSessionResults) -> Promise {
        let session = admin_ui::AdminUiSession::new(self.storage.clone());
        results.get().set_session(ui_session::Client{
            client: web_session::ToClient::new(session)
                .into_client::<::capnp_rpc::Server>()
                .client,
        });
        ok()
    }

    /*
    fn new_session(&mut self,
                   params: ui_view::NewSessionParams,
                   mut results: ui_view::NewSessionResults) -> Promise {
        let lmdb_site = self.get_site("X");
        Promise::from_future(async move {
            let session_type_id = params.get()?.get_session_type();
            if session_type_id != web_session::Client::type_id() {
                return Err(capnp::Error::failed(format!(
                            "unsupported session type id: {}",
                            session_type_id)))
            }
            let site = web_site::ToClient::new(lmdb_site?)
                .into_client::<::capnp_rpc::Server>();
            let session = web_site_session::new(site);
            results.get().set_session(ui_session::Client{
                client: web_session::ToClient::new(session)
                    .into_client::<::capnp_rpc::Server>()
                    .client,
            });
            Ok(())
        })
    }
    */

    fn new_request_session(&mut self,
                           params: ui_view::NewRequestSessionParams,
                           _results: ui_view::NewRequestSessionResults) -> Promise {
        let lmdb_site = self.get_site("X");
        Promise::from_future(async move {
            let params = params.get()?;
            let context = params.get_context()?;

            let site = web_site::ToClient::new(lmdb_site?)
                .into_client::<::capnp_rpc::Server>();
            let mut req = context.fulfill_request_request();
            {
                let mut fulfill_params = req.get();
                let mut fp = fulfill_params.reborrow();
                fp.set_descriptor(params.get_request_info()?.get(0))?;
                fp.get_cap().set_as_capability(site.client.hook);
            }
            let _ = req.send().promise.await?;
            Ok(())
        })
    }
}
