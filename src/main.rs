use tokio;
use futures_util::io::AsyncReadExt;
use capnp_rpc::{
    RpcSystem,
    twoparty,
    rpc_twoparty_capnp,
};
use sandstorm::{
    grain_capnp::{
        ui_view,
        sandstorm_api,
    },
};

use webpub::main_view;


pub fn run_app(uiview: ui_view::Client) -> Result<(), Box<dyn (::std::error::Error)>> {
    use ::std::os::unix::io::{FromRawFd};

    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let local = tokio::task::LocalSet::new();

    local.block_on(&mut rt, async {
        let stream: ::std::os::unix::net::UnixStream = unsafe { FromRawFd::from_raw_fd(3) };
        let stream = tokio::net::UnixStream::from_std(stream)?;
        let (read_half, write_half) = futures_tokio_compat::Compat::new(stream).split();

        let network =
            Box::new(twoparty::VatNetwork::new(read_half, write_half,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));

        let (tx, _rx) = ::futures::channel::oneshot::channel();
        /*
        let sandstorm_api: sandstorm_api::Client<::capnp::any_pointer::Owned> =
            ::capnp_rpc::new_promise_client(rx.map_err(|_e| capnp::Error::failed(format!("oneshot was canceled"))));

            */
        let mut rpc_system = RpcSystem::new(network, Some(uiview.client));

        drop(tx.send(rpc_system.bootstrap::<sandstorm_api::Client<::capnp::any_pointer::Owned>>(
            ::capnp_rpc::rpc_twoparty_capnp::Side::Server).client));

        Ok::<_, Box<dyn (std::error::Error)>>(rpc_system.await?)
    })?;

    Ok(())
}

fn main() {
    run_app(capnp_rpc::new_client(main_view::MainViewImpl::new_from_env().unwrap())).unwrap();
}
