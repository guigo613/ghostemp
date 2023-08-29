use super::{
    mime::Mime,
    Response,
    GenResult,
    Read
};
use std::{
    iter,
    fs::File,
    ffi::OsStr,
    path::Path,
};

pub trait Render {
    fn render(&self) -> GenResult<Response> {
        self.render_with(Default::default())
    }

    fn render_with(&self, headers: Vec<(String, String)>) -> GenResult<Response>;
}

impl Render for Path {
    fn render_with(&self, headers: Vec<(String, String)>) -> GenResult<Response> {
        let mut file = File::open(self)?;
        let mut buf = [0; 2048];
        let mut starter = false;
        let file_size = self.metadata()?.len();
        let mime: Mime = if let Some(m) = headers.iter().position(|(x, _)| x.to_lowercase() == "content-type") {
            headers[m].0.clone().into()
        } else {
            self.extension().and_then(OsStr::to_str).unwrap_or_default().into()
        };

        let response = iter::from_fn(move || {
            if !starter {
                starter = true;
                Some(format!("\
                    HTTP/1.1 200 Ok\r\n\
                    Content-Length: {file_size}\r\n\
                    Content-Type: {mime}\r\n\
                    {}\r\n\
                    \r\n",
                    headers.iter().map(|(x, y)| format!("{x}: {y}\r\n")).collect::<Vec<_>>().join("")).as_bytes().to_vec())
            } else {
                match file.read(&mut buf) {
                    Ok(size) if size > 0 => Some(buf[..size as usize].to_vec()),
                    _ => None
                }
            }
        });

        Ok(Box::new(response))
    }
}

impl<R: AsRef<[u8]>> Render for R {
    fn render_with(&self, headers: Vec<(String, String)>) -> GenResult<Response> {
        let mut file = self.as_ref().to_vec();
        let file_size = file.len();
        let mut starter = false;
        let mime: Mime = if let Some(m) = headers.iter().position(|(x, _)| x.to_lowercase() == "content-type") {
            headers[m].0.clone().into()
        } else {
            Mime::Plain
        };

        let response = iter::from_fn(move || {
            if !starter {
                starter = true;
                Some(format!("\
                    HTTP/1.1 200 Ok\r\n\
                    Content-Length: {file_size}\r\n\
                    Content-Type: {mime}\r\n\
                    {}\r\n\
                    \r\n", headers.iter().map(|(x, y)| format!("{x}: {y}\r\n")).collect::<Vec<_>>().join("")).as_bytes().to_vec())
            } else {
                Some(file.drain(..2048).collect::<Vec<_>>())
            }
        });

        Ok(Box::new(response))
    }
}