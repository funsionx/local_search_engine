pub mod lexer;
use crate::lexer::Lexer;
use std::collections::HashMap;

use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{exit, ExitCode};
use std::{env, io};
use tiny_http::StatusCode;
use tiny_http::{Header, Method, Request, Response, Server};
use xml::reader::XmlEvent;
use xml::EventReader;

fn tf(term: &str, doc: &TermFreq) -> f32 {
    let sum: usize = doc.iter().map(|(_, freq)| *freq).sum();
    let term = *doc.get(term).unwrap_or(&0) as f32;
    term / sum as f32
}

fn read_entire_xml(file_path: &Path) -> Result<String, std::io::Error> {
    let file = File::open(file_path).unwrap();

    let mut content = String::new();

    EventReader::new(file).into_iter().for_each(|e| {
        if let XmlEvent::Characters(text) = e.expect("woooooow") {
            content.push_str(&text);
            content.push(' ')
        }
    });
    Ok(content)
}

type TermFreq = HashMap<String, usize>;
type TermFreqInd = HashMap<PathBuf, TermFreq>;

fn check_index(index_path: &str) -> io::Result<()> {
    let index_file = File::open(index_path)?;
    println!("Reading {index_path} index file...");
    let tf_index: TermFreqInd = serde_json::from_reader(index_file).expect("serde does not fail");
    println!(
        "{index_path} contains {count} files",
        count = tf_index.len()
    );
    Ok(())
}

fn index_folder(dir_path: &str) -> io::Result<()> {
    let dir = fs::read_dir(dir_path)?;
    let mut termfreq_i = TermFreqInd::new();
    for file in dir {
        let path = file?.path();
        println!("Proccesing {path}...", path = path.to_string_lossy());

        let content = read_entire_xml(&path).unwrap().chars().collect::<Vec<_>>();
        let mut termfreq = TermFreq::new();

        Lexer::new(&content).for_each(|term| {
            if let Some(freq) = termfreq.get_mut(&term) {
                *freq += 1;
            } else {
                termfreq.insert(term, 1);
            }
        });

        let mut stats = termfreq.iter().collect::<Vec<_>>();
        stats.sort_by(|(_, a), (_, b)| b.cmp(a));
        stats.reverse();
        termfreq_i.insert(path, termfreq);

        println!("{esc}c", esc = 27 as char);
    }
    let index_path = "index.json";
    println!("Saving {index_path}...");
    let index_file = File::create(index_path)?;
    serde_json::to_writer(index_file, &termfreq_i).expect("err");
    println!("Done!");
    Ok(())
}

fn serve_static_file(
    request: Request,
    file_path: &str,
    content_type: &str,
) -> Result<(), std::io::Error> {
    let content_type_header = Header::from_bytes("Content-Type", content_type).expect("Didnt");

    let file = match File::open(file_path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("ERROR: \n can't serve {file_path}: {error}");
            if error.kind() == io::ErrorKind::NotFound {
                return request
                    .respond(Response::from_string("404").with_status_code(StatusCode(404)));
            }
            return request.respond(Response::from_string("500").with_status_code(StatusCode(500)));
        }
    };

    request.respond(Response::from_file(file).with_header(content_type_header))
}

fn serve_request(mut request: Request, tf_index: &TermFreqInd) -> Result<(), ()> {
    match (request.method(), request.url()) {
        (Method::Post, "/api/search") => {
            let mut buffer = Vec::new();
            request
                .as_reader()
                .read_to_end(&mut buffer)
                .expect("cant fail");
            let body = std::str::from_utf8(&buffer)
                .map_err(|err| eprintln!("{err} occured"))?
                .chars()
                .collect::<Vec<_>>();
            tf_index.into_iter().for_each(|(path, termfreq)| {
                println!("{path}", path = path.display());
                Lexer::new(&body).into_iter().for_each(|word| {
                    println!("{word} => {tf}", tf = tf(&word, termfreq));
                })
            })
        }

        (Method::Get, "/") | (Method::Get, "/index.html") => {
            serve_static_file(request, "index.html", "text/html; charset=utf-8")
                .expect("Error while serving static")
        }
        (Method::Get, "/index.ts") => {
            serve_static_file(request, "index.ts", "text/x.typescript; charset=utf-8")
                .expect("Error while serving typescript")
        }
        _ => request
            .respond(Response::from_string("404").with_status_code(StatusCode(404)))
            .map_err(|err| eprintln!("found {err}"))?,
    }

    Ok(())
}

fn entry() -> Result<(), ()> {
    let mut args = env::args();
    let _program = args.next().expect("path to program is provided");

    let subcommand = args.next().unwrap_or_else(|| {
        println!("ERROR: no subcommand is provided");
        exit(1)
    });

    match subcommand.as_str() {
        "index" => {
            let dir_path = args.next().unwrap_or_else(|| {
                println!("ERROR: no directory is provided for {subcommand} subcommand");
                exit(1);
            });

            index_folder(&dir_path).unwrap_or_else(|err| {
                println!("ERROR: could not index folder {dir_path}: {err}");
                exit(1);
            });
        }
        "search" => {
            let index_path = args.next().unwrap_or_else(|| {
                println!("ERROR: no path to index is provided for {subcommand} subcommand");
                exit(1);
            });
            check_index(&index_path).unwrap_or_else(|err| {
                println!("ERROR: could not check index file {index_path}: {err}");
                exit(1);
            });
        }
        "serve" => {
            let index_path = args.next().ok_or_else(|| {
                eprintln!("ERROR: no path to index is provided for  subcommand");
            })?;
            let index_file = File::open(&index_path)
                .map_err(|err| eprintln!("cant open {index_path} cuz of {err}"))?;
            let tf_index: TermFreqInd =
                serde_json::from_reader(index_file).expect("serde does not fail");
            let address = args.next().unwrap_or("0.0.0.0:8080".to_string());
            let server = Server::http(address.clone())
                .map_err(|err| eprintln!("Cant start server on {address} cuz of {err}"))
                .unwrap();
            println!("Server started on http://{address}");
            server.incoming_requests().for_each(|request| {
                serve_request(request, &tf_index).unwrap();
            })
        }
        _ => {
            println!("ERROR: unknown subcommand {subcommand}");
            exit(1)
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    match entry() {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
//println!("{content}", content = read_entire_xml(dir_path).unwrap())
