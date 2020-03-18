use sandstorm::{
    web_publishing_capnp::web_site,
    grain_capnp::ui_session,
    web_session_capnp::web_session,
};
use capnp;
use capnp::capability::Promise;

pub struct WebSessionImpl {
    client: web_site::Client,
}

pub fn new(client: web_site::Client) -> WebSessionImpl {
    WebSessionImpl { client: client }
}

impl ui_session::Server for WebSessionImpl {
}

impl web_session::Server for WebSessionImpl {
    fn get(&mut self,
           params: web_session::GetParams,
           mut results: web_session::GetResults) -> Promise<(), capnp::Error> {
        let client = self.client.clone();
        Promise::from_future(async move {
            let params = params.get()?;
            let _context = params.get_context()?;
            let ignore_body = params.get_ignore_body();

            let response = results.get();

            let mut req = client.get_entities_request();
            req.get().set_path(params.get_path()?);
            let result = req.send()
                .pipeline.get_entities()
                .get_request().send()
                .promise.await?;
            let value = result.get()?.get_value()?;
            if value.len() == 0 {
                let mut client_error = response.init_client_error();
                client_error.set_status_code(web_session::response::ClientErrorCode::NotFound);
                // TODO: fill in the body.
                Ok(())
            } else {
                let mut content = response.init_content();
                content.set_status_code(web_session::response::SuccessCode::Ok);
                content.set_mime_type("text/plain");
                if !ignore_body {
                    content.get_body().set_bytes(b"Test.");
                }
                Ok::<(), capnp::Error>(())
            }
        })
    }
}

