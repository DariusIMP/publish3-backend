pub mod users;
pub mod authors;
pub mod publications;
pub mod citations;
pub mod publication_authors;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    users::config(cfg);
    authors::config(cfg);
    publications::config(cfg);
    citations::config(cfg);
    publication_authors::config(cfg);
}
