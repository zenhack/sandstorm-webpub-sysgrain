use std::{
    fs,
    io,
    path,
};
use sandstorm::web_publishing_capnp::web_site;

pub enum Error {
    Io(io::Error),
    Capnp(capnp::Error),
    NonUnicodePath,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

impl From<capnp::Error> for Error {
    fn from(e: capnp::Error) -> Self {
        Error::Capnp(e)
    }
}

type Result<T> = core::result::Result<T, Error>;

/// Helper for uploading files into a website.
pub async fn upload_path(path: &path::Path, site: &web_site::Client) -> Result<()> {
    if path.is_dir() {
        upload_dir(path.to_path_buf(), site).await
    } else {
        upload_file(path, site).await
    }
}

async fn upload_dir(path: path::PathBuf,
                    site: &web_site::Client) -> Result<()> {
    // Use an explicit stack to recursively walk the file tree, because
    // I(zenhack) can't figure out how to write a recursive async function.
    let mut stack = vec![path];
    loop {
        match stack.pop() {
            None => break,
            Some(path) => {
                if path.is_dir() {
                    for entry in fs::read_dir(path)? {
                        stack.push(entry?.path())
                    }
                } else {
                    upload_file(&path, site).await?;
                }
            }
        }
    }
    Ok(())
}

async fn upload_file(path: &path::Path, site: &web_site::Client) -> Result<()> {
    upload_file_contents(path, path, site).await
    // TODO: check if the file is index.html, if so upload it as the dir as well.
}

async fn upload_file_contents(file_path: &path::Path,
                              url_path: &path::Path,
                              site: &web_site::Client) -> Result<()> {
    let mut req = site.get_entities_request();
    req.get().set_path(url_path.to_str().ok_or(Error::NonUnicodePath)?);

    let mut req = req.send()
        .pipeline.get_entities()
        .as_setter_request().send()
        // We should be able to pipeline, this, but I(zenhack) am getting an error
        // I don't understand:
        .promise.await?.get()?
        .get_setter()?.set_request();
    let entities = req.get().init_value();
    let entity = entities.get(0);
    entity.get_body().set_bytes(&fs::read(file_path)?);
    req.send().promise.await?.get()?;
    Ok(())
}