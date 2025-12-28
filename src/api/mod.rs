pub mod authors;
pub mod publications;
pub mod users;

#[cfg(test)]
pub mod tests;

pub fn config(cfg: &mut actix_web::web::ServiceConfig) {
    users::config(cfg);
    authors::config(cfg);
    publications::config(cfg);
}
