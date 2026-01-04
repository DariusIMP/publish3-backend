use actix_web::{get, HttpResponse};

#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "ok"
    }))
}

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(health_check);
}
