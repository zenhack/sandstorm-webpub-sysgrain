use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
};
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
            let context = params.get_context()?;
            let ignore_body = params.get_ignore_body();

            let response = results.get();

            let mut req = client.get_entities_request();
            req.get().set_path(params.get_path()?);
            let result = req.send()
                .pipeline.get_entities()
                .get_request().send()
                .promise.await?;
            let value = result.get()?.get_value()?;
            match match_content(value,
                                context.get_accept()?,
                                context.get_accept_encoding()?) {
                Some(ref entity) => {
                    let mut content = response.init_content();
                    content.set_status_code(web_session::response::SuccessCode::Ok);
                    content.set_mime_type(entity.get_mime_type()?);
                    if !ignore_body {
                        // TODO: copy over the body properly.
                        content.get_body().set_bytes(b"Test.");
                    }
                },
                None => {
                    let mut client_error = response.init_client_error();
                    client_error.set_status_code(web_session::response::ClientErrorCode::NotFound);
                    if !ignore_body {
                        // TODO: get the response type & body from the web_site's not found handlers.
                        client_error.set_description_html("404 Not found");
                    }
                }
            };
            Ok(())
        })
    }
}

fn match_content<'a>(entities: capnp::struct_list::Reader<'a, web_site::entity::Owned>,
                 accepted_types: capnp::struct_list::Reader<'a, web_session::accepted_type::Owned>,
                 accepted_encodings: capnp::struct_list::Reader<'a, web_session::accepted_encoding::Owned>
                 ) -> Option<web_site::entity::Reader<'a>> {

    let mut accepted_types: Vec<_> = accepted_types.into_iter().collect();
    accepted_types
        .as_mut_slice()
        .sort_by(|&l, &r| {
            r.get_q_value().partial_cmp(&l.get_q_value()).unwrap_or(Ordering::Equal)
        });

    // TODO: Prioritize results by qValue; right now we just ignore it for
    // Accept-Encoding.
    let accepted_encodings: HashSet<_> = accepted_encodings
        .into_iter()
        .filter_map(|enc| {
            enc.get_content_coding().map(|e| Some(e)).unwrap_or(None)
        })
        .collect();

    let entities: HashMap<_, _> = entities.into_iter().filter_map(|entity| {
        entity.get_mime_type()
            .map(|mime_type| Some((mime_type, entity)))
            .unwrap_or(None)
    }).collect();

    for typ in accepted_types {
        if let Ok(mime_type) = typ.get_mime_type() {
            if let Some(entity) = entities.get(mime_type)  {
                let encoding_ok = entity
                    .get_encoding()
                    .map(|enc| accepted_encodings.contains(enc))
                    .unwrap_or(false);
                if encoding_ok {
                    return Some(*entity);
                }
            }
        }
    }
    return None
}
