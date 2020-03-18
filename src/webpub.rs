use capnp::{Error, capability::Promise};
use capnp_rpc::pry;
use sandstorm::web_publishing_capnp::web_site;

struct WebSiteImpl {
    url: String,
}

impl web_site::Server for WebSiteImpl {
    fn get_url(&mut self,
               _params: web_site::GetUrlParams,
               mut results: web_site::GetUrlResults) -> Promise<(), Error> {
        results.get().set_path(&self.url);
        Promise::ok(())
    }

    fn get_subsite(&mut self,
                   params: web_site::GetSubsiteParams,
                   mut results: web_site::GetSubsiteResults) -> Promise<(), Error> {
        let site = WebSiteImpl {
            url: self.url.clone() + pry!(pry!(params.get()).get_prefix())
        };
        results.get().set_site(web_site::ToClient::new(site)
                               .into_client::<capnp_rpc::Server>());
        Promise::ok(())
    }
}
