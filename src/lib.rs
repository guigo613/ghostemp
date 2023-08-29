mod thread_pool;
mod render;
mod mime;

pub mod prelude {
    pub use crate::render::*;
}

use std::{
    net::{
        TcpListener,
        SocketAddr,
        SocketAddrV4,
        Ipv4Addr
    },
    io::{
        BufWriter,
        Error,
        prelude::*
    },
    ops::Deref,
    collections::HashMap,
    sync::Arc,
    // error::Error
};
use thread_pool::ThreadPool;
pub use mime::Mime;

#[cfg(windows)]
use openssl::ssl::{
    SslMethod,
    SslAcceptor,
    SslFiletype
};
use serde_json::{
    self,
    Value
};

const IP: Ipv4Addr = Ipv4Addr::UNSPECIFIED;

#[cfg(windows)]
static mut ACCEPTOR: Option<SslAcceptor> = None;

const NOT_FOUND: &[u8] = b"HTTP/1.1 404 Not Found\nContent-Length: 9\n\nNot Found";

type GenResult<T> = Result<T, Error>;
type Response = Box<dyn Iterator<Item=Vec<u8>>>;
type Args = HashMap<String, String>;

pub struct ClientHTTP {
    urls: Arc<Urls>,
    pool: ThreadPool
}

impl ClientHTTP {
    pub fn new(urls: Urls) -> Self {
        Self::new_with(urls, 10)
    }

    pub fn new_with(urls: Urls, amnt: usize) -> Self {
        let pool = ThreadPool::new(amnt);
        let urls = Arc::new(urls);

        Self {
            urls,
            pool
        }
    }

    pub fn listen(&self, port: u16) -> GenResult<()> {
        self._listen(IP, port)
    }

    pub fn listen_local(&self, port: u16) -> GenResult<()> {
        self._listen(Ipv4Addr::LOCALHOST, port)
    }

    fn _listen(&self, ip: Ipv4Addr, port: u16) -> GenResult<()> {
        let streams = TcpListener::bind(SocketAddrV4::new(ip, port))?;
        let sender = self.pool.clone();
        let urls = Arc::clone(&self.urls);

        let _ = self.pool.execute(move || {
            for stream in streams.incoming().filter_map(Result::ok) {
                let u = Arc::clone(&urls);
                let socket = stream.peer_addr().ok();

                if let Err(err) = sender.execute(move || {
                    Self::treat_request(stream, u, socket)
                }) {
                    eprintln!("Error: {err}");
                }
            }
        });

        Ok(())
    }
    
    #[cfg(windows)]
    pub fn listen_https(&self, port: u16) -> GenResult<()> {
        let streams = TcpListener::bind(SocketAddrV4::new(IP, port))?;
        let sender = self.pool.clone();
        let urls = Arc::clone(&self.urls);
        let acceptor = get_acceptor();

        let _ = self.pool.execute(move || {
            for stream in streams.incoming() {
                if let Ok((socket, Ok(s))) = stream.map(|s| (s.peer_addr().ok(), acceptor.accept(s))) {
                    let u = Arc::clone(&urls);

                    if let Err(err) = sender.execute(move || {
                        Self::treat_request(s, u, socket)
                    }) {
                        eprintln!("Error: {err}");
                    }
                }
            }
        });

        Ok(())
    }

    fn treat_request<T: Read + Write>(mut stream: T, urls: Arc<Urls>, socket: Option<SocketAddr>) {
        let mut buf = Vec::new();

        let size = stream.read_to_end(&mut buf).unwrap();
        let req = buf[..size].into();

        let page = urls.go(req, socket);

        let mut stream = BufWriter::new(stream);

        for p in page {
            if let Err(_) = stream.write(&p) {
                break;
            };
        }
        let _ = stream.flush();
    }
}

pub struct Urls {
    sub_urls: HashMap<String, Urls>,
    local: Option<Box<dyn Fn(Request, Option<SocketAddr>, Args) -> GenResult<Response> + Send + Sync>>,
    absolute: bool
}

impl Urls {
    pub fn new(absolute: bool) -> Self {
        Self { sub_urls: HashMap::new(), local: None, absolute }
    }

    pub fn append<F: Fn(Request, Option<SocketAddr>, Args) -> GenResult<Response> + Send + Sync + 'static>(&mut self, url: &str, func: F) {
        let mut path = url.split_terminator("/");

        if url.starts_with("/") {
            let _ = path.next();
        }

        let mut urls = self;

        while let Some(p) = path.next() {
            urls = urls.sub_urls.entry(p.to_owned()).or_insert(Urls::default());
        }

        urls.add(func);
    }

    pub fn add<F: Fn(Request, Option<SocketAddr>, Args) -> GenResult<Response> + Send + Sync + 'static>(&mut self, func: F) {
        self.local = Some(Box::new(func))
    }

    fn go(&self, req: Request, socket: Option<SocketAddr>) -> Response {
        let mut path = req.url().split_terminator("/").skip(1);
        let mut args = HashMap::new();
        let mut urls = Some(self);

        while let Some(mut p) = path.next() {
            let urls_ref = urls.unwrap();
            if p.contains("?") {
                let mut splited = p.split("?");
                let temp = splited.next();

                if let Some(query) = splited.next() {
                    for (k, v) in query.split("&").map(|x| x.split_at(x.find("=").unwrap_or_default())) {
                        args.insert(k.to_owned(), v.trim_start_matches("=").to_owned());
                    }
                }

                p = temp.unwrap_or(p);
            }

            match urls_ref.sub_urls.get(p) {
                Some(url) => urls = Some(url),
                _ if urls_ref.absolute => {
                    urls = None;
                    break
                },
                _ => break
            };
        }

        match urls.map_or(None, |u| u.local.as_ref()) {
            Some(ref f) => f(req, socket, args).unwrap_or_else(|err| {
                eprintln!("{err}");
                not_found()
            }),
            _ => not_found()
        }
    }
}

impl Default for Urls {
    fn default() -> Self {
        Self::new(true)
    }
}

#[repr(u8)]
#[derive(Debug)]
pub enum Method {
    GET,
    POST(Data),
    UNDEFINIED
}

#[repr(u8)]
#[derive(Debug)]
pub enum Data {
    Default(HashMap<String, String>),
    Json(Value),
    Xml(String),
    Unknown(String),
    None
}

#[derive(Debug)]
pub struct Request {
    method: Method,
    url: String,
    _protocol: String,
    map: HashMap<String, String>
}

impl Request {
    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn url(&self) -> &str {
        &self.url
    }
}

impl Deref for Request {
    type Target = HashMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

impl From<&[u8]> for Request {
    fn from(req: &[u8]) -> Self {
        match req {
            [0x47 | 0x67, 0x45 | 0x65, 0x54 | 0x74, ..] => {
                let mut lines = req[4..].lines();
                let (url, _protocol) = lines.next().map(|r| r.map(|v| {
                    let mut splited = v.split_whitespace();
                    (splited.next().unwrap_or("/").to_owned(), splited.next().unwrap_or("HTTP/1.1").to_owned())
                }).unwrap_or((String::from("/"), String::from("HTTP/1.1"))))
                .unwrap_or((String::from("/"), String::from("HTTP/1.1")));
                let mut map = HashMap::new();

                for line in lines {
                    if let Ok(l) = line {
                        let idx = if let Some(i) = l.find(":") { i } else { continue };
                        map.insert(l[..idx].to_owned(), l[idx+1..].trim().to_owned());
                    }
                }

                Self { method: Method::GET, url, _protocol, map }
            },
            [0x50 | 0x70, 0x4F | 0x6F, 0x53 | 0x73, 0x54 | 0x74, ..] => {
                let mut content = None;
                let mut post = Data::None;
                let mut lines = req[5..].lines();
                let mut body = false;
                let (url, _protocol) = lines.next().map(|r| r.map(|v| {
                    let mut splited = v.split_whitespace();
                    (splited.next().unwrap_or("/").to_owned(), splited.next().unwrap_or("HTTP/1.1").to_owned())
                }).unwrap_or((String::from("/"), String::from("HTTP/1.1"))))
                .unwrap_or((String::from("/"), String::from("HTTP/1.1")));
                let mut map: HashMap<String, String> = HashMap::new();

                while let Some(line) = lines.next() {
                    match line {
                        Ok(l) if !body => {
                            if l == "" {
                                body = true;
                                content = map.get("content-type").map(|x| x.parse());
                                continue;
                            }
                            
                            let idx = if let Some(i) = l.find(":") { i } else { continue };
                            map.insert(l[..idx].to_lowercase(), l[idx+1..].trim().to_owned());
                        }
                        Ok(l) if body => {
                            match content {
                                Some(Ok(Mime::Plain)) => {
                                    let mut p = HashMap::new();
                                    for mut v in l.split("&").map(|val| val.split("=")) {
                                        let k = v.next().unwrap().to_string();
                                        let v: Vec<_> = v.collect();
                                        p.insert(k, v.join("="));
                                    }

                                    post = Data::Default(p);
                                    break;
                                }
                                Some(Ok(Mime::Json)) => {
                                    let mut json = lines.map(|x| x.unwrap_or_default()).collect::<Vec<String>>();
                                    json.insert(0, l);

                                    post = Data::Json(serde_json::from_str(json.join("\r\n").as_str()).unwrap_or_default());

                                    break;
                                }
                                _ => ()
                            }
                        }
                        _ => ()
                    }
                }

                Self { method: Method::POST(post), url, _protocol, map }
            },
            _ => Self { method: Method::UNDEFINIED, url: String::from("/"), _protocol: String::from("HTTP/1.1"), map: HashMap::new() },
        }
    }
}

fn not_found() -> Box<std::array::IntoIter<Vec<u8>, 1>> {
    Box::new([NOT_FOUND.to_vec()].into_iter())
}

#[cfg(windows)]
pub fn get_acceptor() -> &'static mut SslAcceptor {
    unsafe {
        ACCEPTOR.get_or_insert_with(new_acceptor)
    }
}

#[cfg(windows)]
pub fn new_acceptor() -> SslAcceptor {
    let mut acceptor = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    acceptor.set_private_key_file(r"C:\conf\privkey1.pem", SslFiletype::PEM).unwrap();
    acceptor.set_certificate_chain_file(r"C:\conf\fullchain1.pem").unwrap();
    acceptor.check_private_key().unwrap();

    acceptor.build()
}