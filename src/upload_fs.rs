use crate::shortcuts;
use std::{
    fs,
    io,
    path,
    result,
};
use sandstorm::{
    util_capnp::assignable::setter,
    web_publishing_capnp::web_site,
};

pub enum Error {
    Io(io::Error),
    Capnp(capnp::Error),
    StripPrefix(path::StripPrefixError),
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

impl From<path::StripPrefixError> for Error {
    fn from(e: path::StripPrefixError) -> Self {
        Error::StripPrefix(e)
    }
}

type Result<T> = core::result::Result<T, Error>;

/// Helper for uploading files into a website.
pub async fn upload_path(path: &path::Path, site: &web_site::Client) -> Result<()> {
    if path.is_dir() {
        upload_dir(path, site).await
    } else {
        upload_file(path, path, site).await
    }
}

#[derive(Clone, Copy, Debug)]
struct UrlPath<'a> {
    s: &'a str,
}

impl <'a> UrlPath<'a> {
    fn new(file_root: &'a path::Path, file_path: &'a path::Path) -> Result<Self> {
        let ret = UrlPath {
            s: path_str(file_path.strip_prefix(file_root)?)?
        };
        Ok(ret)
    }

    fn to_str(&self) -> &'a str { self.s }

    fn from_str(s: &'a str) -> Self {
        UrlPath { s: s }
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

async fn upload_dir(root: &path::Path,
                    site: &web_site::Client) -> Result<()> {
    let path = root.to_path_buf();
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
                    upload_file(&root, &path, site).await?;
                }
            }
        }
    }
    Ok(())
}

fn path_str(p: &path::Path) -> Result<&str> {
    p.to_str().ok_or(Error::NonUnicodePath)
}

async fn upload_file(root: &path::Path,
                     path: &path::Path,
                     site: &web_site::Client) -> Result<()> {
    let mime_type = guess_mime_type(path)?;
    if path.ends_with("index.html") {
        let parent = UrlPath::new(root, path.parent().expect("non-empty path"))?;
        let parent_string_with_slash = String::from(parent.to_str()) + "/";
        let parent_with_slash = UrlPath::from_str(&parent_string_with_slash);
        upload_file_contents(&mime_type,
                             path,
                             parent_with_slash,
                             site).await?;
        upload_redirect(parent,
                        parent_with_slash,
                        site).await?;
        upload_redirect(UrlPath::new(root, path)?,
                        parent_with_slash,
                        site).await
    } else {
        upload_file_contents(&mime_type,
                             path,
                             UrlPath::new(root, path)?,
                             site).await
    }
}

async fn setter_for_path<'a>(url_path: UrlPath<'a>, site: &web_site::Client)
    -> result::Result<setter::Client<shortcuts::entity_list::Owned>, capnp::Error>
{
    let mut req = site.get_entities_request();
    req.get().set_path(url_path.to_str());
    req.send()
        .pipeline.get_entities()
        .as_setter_request().send()
        // We should be able to pipeline this, but I(zenhack) am getting an error
        // I don't understand:
        .promise.await?.get()?.get_setter()
}

async fn upload_redirect<'a>(from: UrlPath<'a>,
                             to: UrlPath<'a>,
                             site: &web_site::Client) -> Result<()> {
    let mut req = setter_for_path(from, site).await?.set_request();
    let entities = req.get().initn_value(1);
    let mut entity = entities.get(0);
    entity.set_redirect_to(to.to_str());
    req.send().promise.await?.get()?;
    Ok(())
}

async fn upload_file_contents<'a>(mime_type: &str,
                                  file_path: &path::Path,
                                  url_path: UrlPath<'a>,
                                  site: &web_site::Client) -> Result<()> {
    let mut req = setter_for_path(url_path, site).await?.set_request();
    let entities = req.get().initn_value(1);
    let mut entity = entities.get(0);
    entity.reborrow().get_body().set_bytes(&fs::read(file_path)?);
    entity.set_mime_type(mime_type);
    req.send().promise.await?.get()?;
    Ok(())
}
