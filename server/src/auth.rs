use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub exp: usize,  // expiration timestamp
    pub iat: usize, // issued at
}

pub struct AuthService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthService {
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
        }
    }

    /// Hash a password using argon2
    pub fn hash_password(&self, password: &str) -> Result<String, String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| format!("Failed to hash password: {}", e))
    }

    /// Verify a password against a hash
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, String> {
        let parsed_hash =
            PasswordHash::new(hash).map_err(|e| format!("Invalid password hash: {}", e))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Create a JWT token
    pub fn create_token(&self, user_id: &str, expiration_hours: u64) -> Result<String, String> {
        let now = Instant::now();
        let expiration = now + Duration::from_secs(expiration_hours * 3600);

        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration.exp() as usize,
            iat: now.exp() as usize,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| format!("Failed to create token: {}", e))
    }

    /// Validate a JWT token and return the user_id
    pub fn validate_token(&self, token: &str) -> Option<String> {
        let validation = Validation::default();

        decode::<Claims>(token, &self.decoding_key, &validation)
            .ok()
            .map(|data| data.claims.sub)
    }
}

/// Middleware to require authentication
pub async fn auth_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    // Get Authorization header
    let auth_header = req
        .headers()
        .get(http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s));

    // For now, allow requests without auth for development
    // TODO: Enable strict auth in production
    let _ = auth_header;

    next.run(req).await
}

/// Generate a simple secret from environment or generate one
pub fn get_jwt_secret() -> String {
    std::env::var("CLAUDIA_JWT_SECRET")
        .unwrap_or_else(|_| {
            // Generate a default secret for development
            // In production, this should be set via environment variable
            "claudia-dev-secret-change-in-production".to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_hashing() {
        let auth = AuthService::new("test-secret");
        let hash = auth.hash_password("test_password").unwrap();
        assert!(auth.verify_password("test_password", &hash).unwrap());
        assert!(!auth.verify_password("wrong_password", &hash).unwrap());
    }

    #[test]
    fn test_token_creation() {
        let auth = AuthService::new("test-secret");
        let token = auth.create_token("user123", 24).unwrap();
        let user_id = auth.validate_token(&token).unwrap();
        assert_eq!(user_id, "user123");
    }
}
