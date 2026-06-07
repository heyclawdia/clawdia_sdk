use core::fmt;

#[derive(Clone, Eq, PartialEq)]
/// Supabase service auth used by REST-backed store adapters.
pub struct SupabaseAuth {
    api_key: String,
    bearer_token: String,
}

impl SupabaseAuth {
    /// Creates Supabase REST auth using the same secret for `apikey` and bearer auth.
    pub fn service_role(secret: impl Into<String>) -> Self {
        let secret = secret.into();
        Self {
            api_key: secret.clone(),
            bearer_token: secret,
        }
    }

    /// Creates Supabase REST auth from explicit header values.
    pub fn new(api_key: impl Into<String>, bearer_token: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            bearer_token: bearer_token.into(),
        }
    }

    /// Returns the `apikey` header value.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Returns the bearer token header value.
    pub fn bearer_token(&self) -> &str {
        &self.bearer_token
    }
}

impl fmt::Debug for SupabaseAuth {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SupabaseAuth")
            .field("api_key", &"<redacted>")
            .field("bearer_token", &"<redacted>")
            .finish()
    }
}
