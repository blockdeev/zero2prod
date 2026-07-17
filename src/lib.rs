use actix_web::dev::Server;
use actix_web::{App, HttpResponse, HttpServer, Responder, get, post, web};
use serde::Deserialize;
use std::net::TcpListener;

#[derive(Deserialize)]
#[allow(dead_code)]
struct FormData {
    email: String,
    name: String,
}

#[get("/health_check")]
async fn health_check() -> impl Responder {
    HttpResponse::Ok().finish()
}

#[post("/subscriptions")]
async fn subscribe(_form: web::Form<FormData>) -> impl Responder {
    HttpResponse::Ok().finish()
}

pub fn run(listener: TcpListener) -> Result<Server, std::io::Error> {
    let server = HttpServer::new(|| App::new().service(health_check).service(subscribe))
        .listen(listener)? //bind the port on our own with TcpListener
        .run(); //on main, and hand that over to the HttpServer using listen

    Ok(server)
}
