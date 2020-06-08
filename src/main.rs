use tokio;
use futures_util::io::AsyncReadExt;
use futures_util::future::TryFutureExt;
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
    sandstorm_http_bridge_capnp::sandstorm_http_bridge,
};

use webpub::main_view;


pub fn run_sandstorm_app() {
    let uiview: ui_view::Client = capnp_rpc::new_client(main_view::MainViewImpl::new_from_env().unwrap());
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

        Ok::<_, Box<dyn (std::error::Error)>>(rpc_system.await.unwrap())
    }).unwrap();
}

fn upload_dir(dir: &str, restore: &[u8]) {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    let local = tokio::task::LocalSet::new();

    local.block_on(&mut rt, async {
        let stream = tokio::net::UnixStream::connect(std::path::Path::new("/tmp/sandstorm-api"))
            .await.unwrap();
        let (read_half, write_half) = futures_tokio_compat::Compat::new(stream).split();

        let network =
            Box::new(twoparty::VatNetwork::new(read_half, write_half,
                                               rpc_twoparty_capnp::Side::Client,
                                               Default::default()));
        let (_tx, rx) = ::futures::channel::oneshot::channel();
        let bridge: sandstorm_http_bridge::Client =
            ::capnp_rpc::new_promise_client(rx.map_err(|_e| capnp::Error::failed(format!("oneshot was canceled"))));
        let mut req = bridge
            .get_sandstorm_api_request().send()
            .pipeline.get_api().restore_request();
        let mut token_buf = req.init_token(restore.len());
        token_buf[..].clone_from_slice(restore);
        let cap = req.send().pipeline.get_cap();
        RpcSystem::new(network, None).await.unwrap();
    })
}

fn main() {
    let matches = clap::App::new("Sandstorm Web Publishing")
        .version("0.1")
        .author("Ian Denhardt <ian@zenhack.net>")
        .subcommand(clap::SubCommand::with_name("upload-fs")
                    .about("Upload a local directory as a website.")
                    .arg(clap::Arg::with_name("directory")
                         .short("d")
                         .long("directory")
                         .value_name("PATH")
                         .required(true)
                         .help("The directory to upload"))
                    .arg(clap::Arg::with_name("restore")
                         .short("r")
                         .long("restore")
                         .value_name("RESTORE_TOKEN")
                         .required(true)
                         .help("A token with which to acquire the website capability")))
                    .get_matches();
    if let Some(matches) = matches.subcommand_matches("upload-fs") {
        let dir = matches.value_of("directory").unwrap();
        let restore = matches.value_of("restore").unwrap();
        upload_dir(dir, hex::decode(restore).unwrap())
    } else {
        run_sandstorm_app()
    }
}
