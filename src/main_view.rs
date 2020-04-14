use std::{
    env,
    fs,
    path::PathBuf,
};
use capnp::{traits::HasTypeId};
use sandstorm::{
    grain_capnp::{ui_view, ui_session},
    web_publishing_capnp::web_site,
    web_session_capnp::web_session,
};
use crate::promise_util::{Promise, ok};
use crate::lmdb_web_site;
use crate::web_site_session;

pub struct MainViewImpl {
    site_dir: PathBuf,
}

impl MainViewImpl {
    pub fn new(site_dir: PathBuf) -> MainViewImpl {
        MainViewImpl{
            site_dir: site_dir,
        }
    }

    pub fn new_from_env() -> Result<MainViewImpl, env::VarError> {
        Ok(Self::new(PathBuf::from(env::var("WEB_SITES_DIR")?)))
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
                   params: ui_view::NewSessionParams,
                   mut results: ui_view::NewSessionResults) -> Promise {
        let mut path = self.site_dir.clone();
        path.push("X");
        Promise::from_future(async move {
            let session_type_id = params.get()?.get_session_type();
            if session_type_id != web_session::Client::type_id() {
                return Err(capnp::Error::failed(format!(
                            "unsupported session type id: {}",
                            session_type_id)))
            }
            // Make an effort to create the dir if needed. If this fails,
            // it may be because it already exists, and if it's a "real"
            // failure we'll hit it later anyway, so ignore the result:
            let _ = fs::create_dir_all(&path);
            let lmdb_site = lmdb_web_site::LMDBWebSite::open(
                String::from("site"),
                String::from("http://example.com"),
                &path,
            ).map_err(lmdb_web_site::db_err)?;
            let site = web_site::ToClient::new(lmdb_site)
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
}
