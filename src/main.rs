// VERY basic 'hello, world' version of the relay. I was figuring out Rust. Needs to be
// refactored really badly.
// Requires: https://github.com/getsentry/sentry/tree/feature/relay-config

extern crate backtrace;
extern crate chrono;
extern crate clap;
extern crate env_logger;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate hyper;
#[macro_use] extern crate if_chain;
#[macro_use] extern crate lazy_static;
#[macro_use] extern crate log;
extern crate regex;
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;
extern crate term;

mod constants;
mod errors;

use constants::VERSION;
use errors::*;

use std::env;
use std::io;
use std::str;
use std::sync::mpsc::{channel, Sender, Receiver};
use std::sync::Mutex;
use std::thread;

use clap::{Arg, App, AppSettings};
use hyper::client::Client;
use hyper::header::{Headers, ContentType, ContentLength, Host};
use hyper::method::Method;
use hyper::net::HttpListener;
use hyper::server::{Server, Request, Response, Listening};
use hyper::status::StatusCode;
use hyper::uri::RequestUri;
use regex::Regex;
use serde_json::Value;


struct SimpleLogger<W: ?Sized> {
    f: Mutex<Box<W>>,
}

impl<W: io::Write + Send + ?Sized> log::Log for SimpleLogger<W> {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        metadata.level() <= log::LogLevel::Info
    }

    fn log(&self, record: &log::LogRecord) {
        let mut f = self.f.lock().unwrap();
        if self.enabled(record.metadata()) {
            writeln!(f, "[{}] {} | {}{}",
                     chrono::Local::now(),
                     record.target().split(':').next().unwrap(),
                     match record.level() {
                         log::LogLevel::Error => "ERROR: ",
                         log::LogLevel::Warn => "WARNING: ",
                         _ => "",
                     },
                     record.args()).ok();
        }
    }
}

fn init_backtrace() {
    use backtrace::Backtrace;
    use std::panic;
    use std::thread;

    panic::set_hook(Box::new(|info| {
        let backtrace = Backtrace::new();

        let thread = thread::current();
        let thread = thread.name().unwrap_or("unnamed");

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => {
                match info.payload().downcast_ref::<String>() {
                    Some(s) => &**s,
                    None => "Box<Any>",
                }
            }
        };

        match info.location() {
            Some(location) => {
                println!("thread '{}' panicked at '{}': {}:{}\n\n{:?}",
                         thread,
                         msg,
                         location.file(),
                         location.line(),
                         backtrace);
            }
            None => println!("thread '{}' panicked at '{}'{:?}", thread, msg, backtrace),
        }
    }));
}

#[derive(Debug)]
struct Config {
    bind: String,
    sentry_server: String
}

fn main() {
    init_backtrace();

    let matches =
        App::new("sentry-relay")
            .about("Sentry Relay")
            .version(VERSION)
            .arg(Arg::with_name("bind")
                .help("Bind to a specific address (ip:port)")
                .long("bind")
                .value_name("ADDR")
                .default_value("0.0.0.0:3000"))
            .arg(Arg::with_name("log_level")
                .help("The log level for sentry-cli{n}\
                     (valid levels: TRACE, DEBUG, INFO, WARN, ERROR)")
                .value_name("LOG_LEVEL")
                .long("log-level")
                .takes_value(true))
            .arg(Arg::with_name("sentry-server")
                .help("URL of the Sentry server")
                .long("sentry-server")
                .value_name("URL")
                .default_value("https://sentry.io"))
            .get_matches();

    let default_log_level = log::LogLevelFilter::Warn;
    let log_level = match matches.value_of("log_level") {
        Some(level_str) =>
            match level_str.parse() {
                Ok(level) => level,
                Err(_) => default_log_level
            },
        None => default_log_level
    };

    log::set_logger(|max_log_level| {
        max_log_level.set(log_level);
        Box::new(SimpleLogger { f: Mutex::new(Box::new(io::stderr())) })
    }).ok();

    let bind = matches.value_of("bind").unwrap();
    let sentry_server = matches.value_of("sentry-server").unwrap();
    let config = Config {
        bind: bind.to_owned(),
        sentry_server: sentry_server.to_owned()
    };
    info!("{:?}", config);

    info!("Starting up Relay");
    run_server(config);
    info!("Exiting");
}

// Blocks on a guard returned by `handle`
fn run_server(config: Config) {
    let listener = HttpListener::new(config.bind).unwrap();

    // TODO: pass config down to factory that returns real handler closure?
    Server::new(listener).handle(proxy_handler).unwrap();
}

header! {
    (SentryAuth, "X-Sentry-Auth") => [String]
}

#[derive(Debug, Deserialize, Clone)]
struct Filter {
    field: String,
    regex: String
}

fn get_relay_config(project_id: u32, auth_header: String) -> Vec<Filter> {
    let mut headers = Headers::new();
    headers.set(SentryAuth(auth_header));

    // TODO: config sentry server
    let sentry_server = "http://localhost:8000";
    let url = &format!("{}/api/0/relay/config/{}/", sentry_server, project_id);

    let client = Client::new();
    let mut res = client.get(url)
        .headers(headers)
        .send()
        .unwrap();

    let mut response_body_bytes: Vec<u8> = Vec::new();
    ::std::io::copy(&mut res, &mut response_body_bytes).unwrap();
    let response_body = str::from_utf8(&response_body_bytes).unwrap();

    info!("Fetched relay config from Sentry: {}", response_body);

    let parsed_body: Value = serde_json::from_str(response_body).unwrap();

    let mut out_filters = vec!();
    if let Value::Array(ref filters) = parsed_body["filters"] {
        for filter in filters {
            let filter: Filter = serde_json::from_value(filter.clone()).unwrap();
//            info!("filter: {:?}", filter);
            out_filters.push(filter)
        }
    }

    out_filters
}

fn proxy_handler(mut req: Request, mut resp: Response) {
    // TODO: verify path/method
    // TODO: verify json/encoding
    // TODO: unpack gzip/base64
    // TODO: hyper conn pooling? https://hyper.rs/hyper/v0.10.7/hyper/client/pool/index.html
    // TODO: hyper client can be reused among threads according to docs (Arc+clone)
    // TODO: decouple proxy request with channel? -- always return 200 for valid json...?
    // TODO: make destination sentry configurable
    // TODO: hit sentry for filter data (every N seconds) -- /api/0/relay/config/
    // TODO: respect 429 or disabled DSN
    // TODO: add self-update like sentry-cli has

    lazy_static! {
        static ref STORE_PATH_RE: Regex = Regex::new(r"^/api/(\d+)/store/$").unwrap();
    }

    let absolute_path = if_chain! {
        if let RequestUri::AbsolutePath(ref uri) = req.uri;
        if STORE_PATH_RE.is_match(uri);
        then {
            uri.clone()
        } else {
            // TODO: write error body???
            return
        }
    };
//    info!("absolute_path: {}", absolute_path);

    // TODO: there must be a better way to do this?
    let mut request_body_bytes: Vec<u8> = Vec::new();
    ::std::io::copy(&mut req, &mut request_body_bytes).unwrap();
    let request_body = str::from_utf8(&request_body_bytes).unwrap();
    let parsed_body: Value = serde_json::from_str(request_body).unwrap();

    info!("Sentry store request received: '{}...'", &request_body[0..100]);

    let mut headers = req.headers;
    // TODO: use configured url, or just clear header and let hyper handle (if it does)?
    headers.set(::hyper::header::Host { hostname: "sentry.io".to_owned(), port: None });

    //    if let Some(&SentryAuth(auth)) = headers.get() {
    //        get_relay_config(1, auth);
    //    };
    
    // TODO: get from request
    let auth_header = "Sentry sentry_version=6,sentry_client=raven-java/8.0.2-beb4b,sentry_key=XXX,sentry_secret=XXX"
    let filters = get_relay_config(1, auth_header.to_owned());
    let mut send = true;
    for filter in filters {
        let re = Regex::new(&filter.regex).unwrap();

        if let Value::String(ref field) = parsed_body[filter.field.clone()] {
            info!("Field value: {}={:?}", filter.field.clone(), field);
            if re.is_match(field) {
                info!("Filter matches, dropping this event.");
                send = false;
            }
        }
    }

    if send {
        // TODO: config sentry server
        //    let sentry_url = "http://sentry.io";
        let sentry_url = "http://localhost:8000";

        let client = Client::new();
        let res = client.post(&format!("{}{}", sentry_url, absolute_path))
            .headers(headers.clone()) // TODO: clone...?
            .body(parsed_body.to_string().as_bytes())
            .send()
            .unwrap();

        info!("Sentry response code status: {}", res.status);
    }

//    info!("request absolute_path: {:?}", req.uri);
//    info!("headers: {:?}", headers);

    let resp_body = "{}";
    *resp.status_mut() = StatusCode::Ok;
    resp.headers_mut().set(::hyper::header::Server(format!("sentry-relay/{}", VERSION)));
    resp.headers_mut().set(ContentLength(resp_body.len() as u64));
    resp.headers_mut().set(ContentType::json());
    resp.send(resp_body.as_bytes()).unwrap();
}
