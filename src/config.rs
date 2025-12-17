fn get_env_var(var_name: &str) -> String {
    std::env::var(var_name).unwrap_or_else(|_| panic!("{} must be set", var_name))
}

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub client_origin: String,

    pub server_address: String,
    pub server_port: String,
    pub server_base_url: String,

    pub s3_access_key: String,
    pub s3_secret_key: String,
    pub s3_endpoint: String,

    // Movement blockchain configuration
    pub movement_network: String,
    pub movement_rpc_url: String,
    pub contract_address: String,

    // Privy authentication
    pub privy_app_id: String,
    pub privy_app_secret: String,
    pub privy_jwt_verification_key: Vec<u8>,

    // Backend signing key for capabilities
    pub backend_private_key: String,
    pub backend_public_key: String,
}

impl Config {
    pub fn init() -> Config {
        let database_url = get_env_var("DATABASE_URL");
        let redis_url = get_env_var("REDIS_URL");
        let client_origin = get_env_var("CLIENT_ORIGIN");

        let server_address = get_env_var("SERVER_ADDRESS");
        let server_port = get_env_var("SERVER_PORT");
        let server_base_url = get_env_var("SERVER_BASE_URL");

        let s3_access_key = get_env_var("S3_ACCESS_KEY");
        let s3_secret_key = get_env_var("S3_SECRET_KEY");
        let s3_endpoint = get_env_var("S3_ENDPOINT");

        // Movement blockchain configuration
        let movement_network = get_env_var("MOVEMENT_NETWORK");
        let movement_rpc_url = get_env_var("MOVEMENT_RPC_URL");
        let contract_address = get_env_var("CONTRACT_ADDRESS");

        // Privy configuration
        let privy_app_id = get_env_var("PRIVY_APP_ID");
        let privy_app_secret = get_env_var("PRIVY_APP_SECRET");
        let privy_jwt_verification_key = base64::decode(get_env_var("PRIVY_JWT_VERIFICATION_KEY"))
            .unwrap_or_else(|_| panic!("PRIVY_JWT_VERIFICATION_KEY must be a valid base64 string"))
            .to_vec();

        // Backend signing key for capabilities
        let backend_private_key = get_env_var("BACKEND_PRIVATE_KEY");
        let backend_public_key = get_env_var("BACKEND_PUBLIC_KEY");

        Config {
            database_url,
            redis_url,
            client_origin,
            server_address,
            server_port,
            server_base_url,
            s3_access_key,
            s3_secret_key,
            s3_endpoint,
            movement_network,
            movement_rpc_url,
            contract_address,
            privy_app_id,
            privy_app_secret,
            privy_jwt_verification_key,
            backend_private_key,
            backend_public_key,
        }
    }
}
