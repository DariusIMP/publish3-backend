use std::future::{Ready, ready};

use actix_web::{
    Error, HttpMessage,
    dev::{Service, ServiceRequest, ServiceResponse, Transform, forward_ready},
};
use futures_util::future::LocalBoxFuture;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::CONFIG;

lazy_static! {
    static ref VALIDATION: Validation = {
        let mut validation = Validation::new(Algorithm::ES256);
        validation.set_audience(&[&CONFIG.privy_app_id]);
        validation.set_issuer(&["privy.io"]);
        validation
    };
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivyClaims {
    /// The user's current session ID
    pub sid: String,
    /// The authenticated user's Privy DID
    pub sub: String,
    /// Your Privy app ID
    pub aud: String,
    /// This will always be 'privy.io'
    pub iss: String,
    /// Unix timestamp for when the access token was signed by Privy
    pub iat: u64,
    /// Unix timestamp for when the access token will expire
    pub exp: u64,
}

// Helper function to extract Privy claims from request extensions
pub fn get_privy_claims(req: &actix_web::HttpRequest) -> Option<PrivyClaims> {
    req.extensions().get::<PrivyClaims>().cloned()
}

// Helper function to verify a Privy token
pub fn verify_privy_token(token: &str) -> Result<PrivyClaims, jsonwebtoken::errors::Error> {
    let decoding_key = DecodingKey::from_ec_pem(&CONFIG.privy_jwt_verification_key)?;

    jsonwebtoken::decode::<PrivyClaims>(token, &decoding_key, &VALIDATION)
        .map(|token_data| token_data.claims)
}

pub struct Privy;

impl<S, B> Transform<S, ServiceRequest> for Privy
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = PrivyMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PrivyMiddleware { service }))
    }
}

pub struct PrivyMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for PrivyMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let auth_header = req.headers().get("Authorization");

        if let Some(auth_header) = auth_header {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..]; // Remove "Bearer " prefix

                    match verify_privy_token(token) {
                        Ok(claims) => {
                            req.extensions_mut().insert(claims);
                            let fut = self.service.call(req);
                            return Box::pin(async move {
                                let res = fut.await?;
                                Ok(res)
                            });
                        }
                        Err(err) => {
                            tracing::warn!("Invalid Privy token: {}", err);
                        }
                    }
                }
            }
        }

        let error = actix_web::error::ErrorUnauthorized(serde_json::json!({
            "error": "Unauthorized",
            "message": "Valid Privy authentication token required"
        }));

        Box::pin(async move { Err(error) })
    }
}
