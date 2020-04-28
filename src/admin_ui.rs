use askama::Template;
use sandstorm::{
    web_session_capnp::web_session,
    grain_capnp::ui_session,
};
use capnp::capability::Promise;
use std::{
    sync::{Arc, Mutex}
};
use crate::storage::Storage;

pub struct AdminUiSession {
    storage: Arc<Mutex<Storage>>,
}

impl AdminUiSession {
    pub fn new(storage: Arc<Mutex<Storage>>) -> Self {
        AdminUiSession{ storage: storage }
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
}

#[derive(Debug, Template)]
#[template(path = "index.html")]
struct Index {
    sites: Vec<String>,
}
