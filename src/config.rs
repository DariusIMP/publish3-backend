use base64::{Engine, engine::general_purpose};

fn get_env_var(var_name: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| panic!("{} must be set", var_name))
}
use p256::pkcs8::DecodePrivateKey;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,

    #[cfg(not(feature = "aws-s3"))]
    pub s3_access_key: String,
    #[cfg(not(feature = "aws-s3"))]
    pub s3_secret_key: String,
    #[cfg(not(feature = "aws-s3"))]
    pub s3_endpoint: String,

    // Movement blockchain configuration
    pub movement_network: String,
    pub movement_rpc_url: String,
    pub movement_indexer_url: String,
    pub contract_address: String,

    // Privy authentication
    pub privy_app_id: String,
    pub privy_app_secret: String,
    pub privy_jwt_verification_key: Vec<u8>,
    pub privy_signer_key: String,
    pub privy_wallet_auth: String,

    // Backend signing key for capabilities
    pub backend_private_key: String,
    pub backend_public_key: String,
}

impl Config {
    pub fn init() -> Config {
        let database_url = get_env_var("DATABASE_URL");

        #[cfg(not(feature = "aws-s3"))]
        let s3_access_key = get_env_var("S3_ACCESS_KEY");
        #[cfg(not(feature = "aws-s3"))]
        let s3_secret_key = get_env_var("S3_SECRET_KEY");
        #[cfg(not(feature = "aws-s3"))]
        let s3_endpoint = get_env_var("S3_ENDPOINT");

        // Movement blockchain configuration
        let movement_network = get_env_var("MOVEMENT_NETWORK");
        let movement_rpc_url = get_env_var("MOVEMENT_RPC_URL");
        let movement_indexer_url = get_env_var("MOVEMENT_INDEXER_URL");
        let contract_address = get_env_var("CONTRACT_ADDRESS");

        // Privy configuration
        let privy_app_id = get_env_var("PRIVY_APP_ID");
        let privy_app_secret = get_env_var("PRIVY_APP_SECRET");
        let privy_jwt_verification_key = general_purpose::STANDARD
            .decode(get_env_var("PRIVY_JWT_VERIFICATION_KEY"))
            .unwrap_or_else(|_| panic!("PRIVY_JWT_VERIFICATION_KEY must be a valid base64 string"))
            .to_vec();
        let privy_signer_key =
            privy_der_base64_to_sec1_pem(get_env_var("PRIVY_SIGNER_KEY")).unwrap();
        let privy_wallet_auth = get_env_var("PRIVY_WALLET_AUTH");

        // Backend signing key for capabilities
        let backend_private_key = get_env_var("BACKEND_PRIVATE_KEY");
        let backend_public_key = get_env_var("BACKEND_PUBLIC_KEY");

        Config {
            database_url,
            #[cfg(not(feature = "aws-s3"))]
            s3_access_key,
            #[cfg(not(feature = "aws-s3"))]
            s3_secret_key,
            #[cfg(not(feature = "aws-s3"))]
            s3_endpoint,
            movement_network,
            movement_rpc_url,
            movement_indexer_url,
            contract_address,
            privy_app_id,
            privy_app_secret,
            privy_jwt_verification_key,
            privy_signer_key,
            privy_wallet_auth,
            backend_private_key,
            backend_public_key,
        }
    }
}

pub fn privy_der_base64_to_sec1_pem(
    der_base64: String,
) -> Result<String, Box<dyn std::error::Error>> {
    // 1. Base64 → PKCS#8 DER
    let pkcs8_der = general_purpose::STANDARD.decode(der_base64)?;

    // 2. Parse PKCS#8 → SecretKey
    let secret_key = p256::SecretKey::from_pkcs8_der(&pkcs8_der)?;

    // 3. Convert → SEC1 PEM
    let sec1_pem = secret_key.to_sec1_pem(Default::default())?;

    Ok(sec1_pem.to_string())
}
