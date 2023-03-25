use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::str;
use tiny_http::{Header, Method, Request, Response, StatusCode};

use crate::lexer::Lexer;

fn serve_404(request: Request) -> io::Result<()> {
    request.respond(Response::from_string("404").with_status_code(StatusCode(404)))
}

fn serve_500(request: Request) -> io::Result<()> {
    request.respond(Response::from_string("500").with_status_code(StatusCode(500)))
}

// fn serve_400(request: Request) -> io::Result<()> {
//     request.respond(Response::from_string("400").with_status_code(StatusCode(400)))
// }

fn tf(term: &str, doc: &TermFreq) -> f32 {
    let sum: usize = doc.iter().map(|(_, freq)| *freq).sum();
    let term = *doc.get(term).unwrap_or(&0) as f32;
    term / sum as f32
}

fn serve_static_file(request: Request, file_path: &str, content_type: &str) -> io::Result<()> {
    let content_type_header = Header::from_bytes("Content-Type", content_type).expect("Didnt");

    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("ERROR: \n can't serve {file_path}: {error}");
            if error.kind() == io::ErrorKind::NotFound {
                return serve_404(request);
            }
            return serve_500(request);
        }
    };

    request.respond(Response::from_file(file).with_header(content_type_header))
}
type TermFreq = HashMap<String, usize>;
type TermFreqInd = HashMap<PathBuf, TermFreq>;

pub fn serve_request(mut request: Request, tf_index: &TermFreqInd) -> io::Result<()> {
    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            let mut buffer = Vec::new();
            request
                .as_reader()
                .read_to_end(&mut buffer)
                .expect("cant fail");
            let body = std::str::from_utf8(&buffer)
                .map_err(|err| eprintln!("{err} occured"))
                .unwrap()
                .chars()
                .collect::<Vec<_>>();
            let mut result = Vec::new();
            tf_index.into_iter().for_each(|(path, termfreq)| {
                let mut total_termfreq = 0_f32;
                Lexer::new(&body)
                    .into_iter()
                    .for_each(|word| total_termfreq += tf(&word, termfreq));
                result.push((path, total_termfreq))
            });
            result.sort_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap());
            result.reverse();
            result.into_iter().for_each(|(a, b)| println!("In {a} occured {b} times", a = a.to_string_lossy()))
        }

        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, "index.html", "text/html; charset=utf-8")
                .map_err(|err| eprintln!("{err} occured"))
                .unwrap()
        }
        (Method::Get, "/index.ts") => {
            serve_static_file(request, "index.ts", "text/x.typescript; charset=utf-8")
                .map_err(|err| eprintln!("{err} occured"))
                .unwrap()
        }
        _ => request
            .respond(Response::from_string("404").with_status_code(StatusCode(404)))
            .map_err(|err| eprintln!("{err} occured"))
            .unwrap(),
    }
    Ok(())
}
