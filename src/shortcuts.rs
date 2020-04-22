pub mod entity_list {
    use sandstorm::web_publishing_capnp::web_site;
    pub type Owned = capnp::struct_list::Owned<web_site::entity::Owned>;
    pub type Reader<'a> = capnp::struct_list::Reader<'a, web_site::entity::Owned>;
}
