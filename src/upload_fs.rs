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

fn guess_mime_type(path: &path::Path) -> Result<String> {
    let ext = match path.extension() {
        None => {
            return Ok(String::from("application/octet-stream"))
        },
        Some(os_str) => {
            os_str.to_str().ok_or(Error::NonUnicodePath)?
        }
    };
    let mime = mime_guess::from_ext(ext).first_or_octet_stream();
    Ok(String::from(mime.essence_str()))
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

fn path_str(p: &path::Path) -> Result<&str> {
    p.to_str().ok_or(Error::NonUnicodePath)
}

async fn upload_file(path: &path::Path, site: &web_site::Client) -> Result<()> {
    let mime_type = guess_mime_type(path)?;
    if path.ends_with("index.html") {
        let parent = path_str(path.parent().expect("non-empty path"))?;
        let parent_with_slash = String::from(parent) + "/";
        upload_file_contents(&mime_type,
                             path,
                             &parent_with_slash,
                             site).await?;
        upload_redirect(parent, &parent_with_slash, site).await?;
        upload_redirect(path_str(path)?, &parent_with_slash, site).await
    } else {
        upload_file_contents(&mime_type,
                             path,
                             path_str(path)?,
                             site).await
    }
}

async fn upload_redirect(from: &str, to: &str, site: &web_site::Client) -> Result<()> {
    // TODO: implement.
    Ok(())
}

async fn upload_file_contents(mime_type: &str,
                              file_path: &path::Path,
                              url_path: &str,
                              site: &web_site::Client) -> Result<()> {
    let mut req = site.get_entities_request();
    req.get().set_path(url_path);

    let mut req = req.send()
        .pipeline.get_entities()
        .as_setter_request().send()
        // We should be able to pipeline, this, but I(zenhack) am getting an error
        // I don't understand:
        .promise.await?.get()?
        .get_setter()?.set_request();
    let entities = req.get().initn_value(1);
    let mut entity = entities.get(0);
    entity.reborrow().get_body().set_bytes(&fs::read(file_path)?);
    entity.set_mime_type(mime_type);
    req.send().promise.await?.get()?;
    Ok(())
}
