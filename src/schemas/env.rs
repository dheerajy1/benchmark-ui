use serde::Deserialize;
use validator::Validate;

#[derive(Clone, Debug, Deserialize, Validate)]
pub struct EnvSchema {
    // Port the API listens on
    #[validate(range(min = 1, message = "APP_PORT must be a valid port number"))]
    pub app_port: u16,

    // Base URL of benchmark-api, compiled into the bundle, required only for the production build
    #[validate(length(min = 1, message = "API_BASE_URL must not be empty"))]
    pub api_base_url: Option<String>,

    // Health check client identifier
    #[validate(length(min = 1, max = 50, message = "CLIENT_ID must be 1-50 characters"))]
    pub client_id: Option<String>,

    // Health check client secret
    #[validate(length(min = 1, max = 50, message = "CLIENT_SECRET must be 1-50 characters"))]
    pub client_secret: Option<String>,
}
