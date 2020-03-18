use capnp::{traits::HasTypeId};
use capnp_rpc::pry;
use sandstorm::{
    grain_capnp::{ui_view, ui_session},
    web_publishing_capnp::web_site,
    web_session_capnp::web_session,
};
use crate::promise_util::{Promise, ok};

pub struct MainViewImpl {
}

impl MainViewImpl {
    pub fn new() -> MainViewImpl {
        MainViewImpl{}
    }
}

pub struct WebSessionImpl {
}

impl ui_session::Server for WebSessionImpl {}

impl web_session::Server for WebSessionImpl {

    fn get(&mut self,
           params: web_session::GetParams,
           mut results: web_session::GetResults) -> Promise {

        let params = pry!(params.get());
        let ignore_body = params.get_ignore_body();
        let response = results.get();
        let mut content = response.init_content();
        content.set_status_code(web_session::response::SuccessCode::Ok);
        content.set_mime_type("text/plain");
        if !ignore_body {
            content.get_body().set_bytes(b"Test.");
        }
        ok()
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
        results.get().set_session(ui_session::Client{
            client: web_session::ToClient::new(WebSessionImpl{})
                .into_client::<::capnp_rpc::Server>()
                .client,
        });
        ok()
    }
}
