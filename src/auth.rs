use tonic::{Request, Status};
use tracing::{debug, warn};

/// Metadata key for user ID
pub const USER_ID_KEY: &str = "x-user-id";
/// Metadata key for tenant ID
pub const TENANT_ID_KEY: &str = "x-tenant-id";
/// Metadata key for authorization header
pub const AUTH_HEADER_KEY: &str = "authorization";

/// Authentication context extracted from request
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub tenant_id: Option<String>,
    pub token: String,
}

/// Authentication interceptor for gRPC requests
#[derive(Clone)]
pub struct AuthInterceptor {
    /// URL of the external authentication service
    auth_service_url: String,
    /// Whether to skip auth in development mode
    skip_auth: bool,
}

impl AuthInterceptor {
    pub fn new(auth_service_url: String, skip_auth: bool) -> Self {
        Self {
            auth_service_url,
            skip_auth,
        }
    }

    /// Extract and validate authentication from request
    pub async fn authenticate<T>(&self, request: &Request<T>) -> Result<AuthContext, Status> {
        // In development mode, optionally skip authentication
        if self.skip_auth {
            debug!("Skipping authentication in development mode");
            return Ok(AuthContext {
                user_id: "dev-user".to_string(),
                tenant_id: Some("dev-tenant".to_string()),
                token: "dev-token".to_string(),
            });
        }

        // Extract authorization header
        let auth_header = request
            .metadata()
            .get(AUTH_HEADER_KEY)
            .ok_or_else(|| Status::unauthenticated("Missing authorization header"))?;

        let auth_str = auth_header
            .to_str()
            .map_err(|_| Status::unauthenticated("Invalid authorization header"))?;

        // Extract bearer token
        let token = if auth_str.starts_with("Bearer ") {
            &auth_str[7..]
        } else {
            return Err(Status::unauthenticated("Invalid authorization format"));
        };

        // Validate with external auth service
        let auth_context = self.validate_token(token).await?;

        Ok(auth_context)
    }

    /// Validate token with external authentication service
    async fn validate_token(&self, token: &str) -> Result<AuthContext, Status> {
        // TODO: Implement actual validation with DataCurve/Shipd auth service
        // For now, this is a placeholder that demonstrates the pattern
        
        // In a real implementation, this would:
        // 1. Call the external auth service to validate the token
        // 2. Extract user_id and tenant_id from the response
        // 3. Cache the result for performance
        
        warn!("Token validation not yet implemented - using placeholder");
        
        // Placeholder implementation
        if token == "invalid" {
            return Err(Status::unauthenticated("Invalid token"));
        }

        Ok(AuthContext {
            user_id: "placeholder-user".to_string(),
            tenant_id: Some("placeholder-tenant".to_string()),
            token: token.to_string(),
        })
    }
}

/// Extension trait to inject auth context into requests
pub trait RequestExt {
    fn auth_context(&self) -> Result<&AuthContext, Status>;
}

impl<T> RequestExt for Request<T> {
    fn auth_context(&self) -> Result<&AuthContext, Status> {
        self.extensions()
            .get::<AuthContext>()
            .ok_or_else(|| Status::internal("Missing authentication context"))
    }
}

/// Tower service for authentication
#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    interceptor: AuthInterceptor,
}

impl<S> AuthService<S> {
    pub fn new(inner: S, interceptor: AuthInterceptor) -> Self {
        Self { inner, interceptor }
    }
}

// We'll implement the Tower service trait for AuthService in the main grpc module