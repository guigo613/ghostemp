use super::{
    mime::Mime,
    Response,
    GenResult,
    Read
};
use std::{
    iter,
    slice,
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
    fn render_with(&self, mut headers: Vec<(String, String)>) -> GenResult<Response> {
        let mut file = File::open(self)?;
        let mut buf = [0; 2048];
        let mut starter = false;
        let file_size = self.metadata()?.len();
        let mime: Mime = if let Some(m) = headers.iter().position(|(x, _)| x.to_lowercase() == "content-type") {
            headers.remove(m).1.clone().into()
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
                    {}\r\n",
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
    fn render_with(&self, mut headers: Vec<(String, String)>) -> GenResult<Response> {
        let file = self.as_ref();
        let mut file_size = file.len();
        let mut file = file.as_ptr() as *mut u8;
        let mut starter = false;
        let mime: Mime = if let Some(m) = headers.iter().position(|(x, _)| x.to_lowercase() == "content-type") {
            headers.remove(m).1.clone().into()
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
                    {}\r\n", headers.iter().map(|(x, y)| format!("{x}: {y}\r\n")).collect::<Vec<_>>().join("")).as_bytes().to_vec())
            } else {
                if file_size > 0 {
                    let end_size = file_size.min(2048);
                    file_size -= end_size;
                    let slice = unsafe { slice::from_raw_parts_mut(file, end_size) };
                    file = unsafe { (slice.last_mut().unwrap() as *mut u8).add(1) };
                    Some(slice.to_vec())
                } else { None }
            }
        });

        Ok(Box::new(response))
    }
}