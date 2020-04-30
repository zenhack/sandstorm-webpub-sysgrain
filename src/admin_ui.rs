use askama::Template;
use sandstorm::{
    web_session_capnp::web_session,
    grain_capnp::{ui_session, session_context},
    web_publishing_capnp::web_site,
};
use capnp::capability::Promise;
use std::{
    sync::{Arc, Mutex}
};
use crate::{
    storage::Storage,
    web_site_session,
    lmdb_web_site,
};

pub struct AdminUiSession {
    storage: Arc<Mutex<Storage>>,
    session_ctx: session_context::Client,
}

impl AdminUiSession {
    pub fn new(storage: Arc<Mutex<Storage>>,
               session_ctx: session_context::Client) -> Self {
        AdminUiSession{
            storage: storage,
            session_ctx: session_ctx,
        }
    }
}

impl ui_session::Server for AdminUiSession {
}

impl web_session::Server for AdminUiSession {
    fn get(&mut self,
           params: web_session::GetParams,
           mut results: web_session::GetResults) -> Promise<(), capnp::Error> {
        let storage = self.storage.clone();
        Promise::from_future(async move {
            let params = params.get()?;
            let path = params.get_path()?;
            let ignore_body = params.get_ignore_body();
            match path {
                "" => {
                    match storage.lock().unwrap().list_sites() {
                        Ok(sites) => {
                            let mut content = results.get().init_content();
                            content.set_status_code(web_session::response::SuccessCode::Ok);
                            content.set_mime_type("text/html");
                            if !ignore_body {
                                let body = Index{ sites: sites }.render().unwrap();
                                content.get_body().set_bytes(body.as_bytes());
                            }
                        },
                        Err(err) => {
                            println!("Error listing sites: {:?}", err);
                            let mut server_error = results.get().init_server_error();
                            if !ignore_body {
                                server_error.set_description_html(
                                    include_str!("../static/server-error.html")
                                );
                            }
                        }
                    };
                    Ok(())
                },
                "admin-ui.js" => {
                    let mut content = results.get().init_content();
                    content.set_status_code(web_session::response::SuccessCode::Ok);
                    content.set_mime_type("text/html");
                    if !ignore_body {
                        content.get_body().set_bytes(
                            include_str!("../static/admin-ui.js").as_bytes()
                        );
                    }
                    Ok(())
                },
                _ => {
                    let mut client_error = results.get().init_client_error();
                    client_error.set_status_code(web_session::response::ClientErrorCode::NotFound);
                    if !ignore_body {
                        client_error.set_description_html(
                            include_str!("../static/not-found.html")
                        );
                    }
                    Ok(())
                },
            }
        })
    }

    fn post(&mut self,
            params: web_session::PostParams,
            mut results: web_session::PostResults) -> Promise<(), capnp::Error> {
        let storage = self.storage.clone();
        let session_ctx = self.session_ctx.clone();
        Promise::from_future(async move {
            let params = params.get()?;
            let path = params.get_path()?;
            match path {
                "offer-site" => {
                    let content = params.get_content()?.get_content()?;
                    let content_str = std::str::from_utf8(content)?;
                    let lmdb_site = storage.lock().unwrap().get(content_str)
                        .map_err(lmdb_web_site::db_err)?;
                    let site = web_site::ToClient::new(lmdb_site)
                        .into_client::<::capnp_rpc::Server>();
                    let session = web_site_session::new(site);
                    let mut req = session_ctx.offer_request();
                    req.get().get_cap().set_as_capability(
                        web_session::ToClient::new(session)
                            .into_client::<::capnp_rpc::Server>()
                            .client
                            .hook
                    );
                    // TODO: set the other parameter fields.
                    req.send().promise.await?;
                    Ok(())
                },
                _ => {
                    // TODO(cleanup): dedup from get()
                    let mut client_error = results.get().init_client_error();
                    client_error.set_status_code(web_session::response::ClientErrorCode::NotFound);
                    client_error.set_description_html(
                        include_str!("../static/not-found.html")
                    );
                    Ok(())
                },
            }
        })
    }
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct Index {
    sites: Vec<String>,
}
