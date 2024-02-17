use actix_files::NamedFile;
use actix_web::{get, web::scope, Responder, Scope};

#[get("/")]
async fn home() -> impl Responder {
    NamedFile::open_async("./static/index.html").await.unwrap()
}

pub fn register_main_service() -> Scope {
    scope("/api").service(home)
}
