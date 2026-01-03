pub mod routes;

use actix_web::web;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/purchases")
            .service(routes::list_user_purchases)
            .service(routes::count_purchases)
            .service(routes::simulate_purchase)
    );
}
