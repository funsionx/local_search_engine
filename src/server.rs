use super::model::*;
use std::fs::File;
use std::io;
use std::str;
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

fn serve_404(request: Request) -> io::Result<()> {
    request.respond(Response::from_string("404").with_status_code(StatusCode(404)))
}

fn serve_500(request: Request) -> io::Result<()> {
    request.respond(Response::from_string("500").with_status_code(StatusCode(500)))
}

fn serve_400(request: Request) -> io::Result<()> {
    request.respond(Response::from_string("400").with_status_code(StatusCode(400)))
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
                return serve_404(request);
            }
            return serve_500(request);
        }
    };

    request.respond(Response::from_file(file).with_header(content_type_header))
}

fn serve_request(model: &impl Model, request: Request) -> io::Result<()> {
    println!("Request!! \n method: {:?} \n url: {:?}")
}
