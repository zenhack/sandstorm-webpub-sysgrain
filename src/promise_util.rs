use capnp::capability;

pub type Promise = capability::Promise<(), ::capnp::Error>;

pub fn ok() -> Promise {
    capability::Promise::ok(())
}
