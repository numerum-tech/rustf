use crate::models::users::Users;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::Utc;
use serde::Serialize;
use std::borrow::Cow;
use std::collections::HashMap;

pub struct AuthService;

#[derive(Debug, Clone, Serialize)]
pub struct AuthenticatedUser {
    pub id: i64,
    pub email: String,
    pub display_name: String,
}

impl AuthService {
    pub async fn register(form: &HashMap<String, String>) -> rustf::Result<AuthenticatedUser> {
        let display_name = required_field(form, "display_name")?;
        let email = normalize_email(required_field(form, "email")?);
        let password = required_field(form, "password")?;
        let password_confirm = required_field(form, "password_confirm")?;

        if password.len() < 8 {
            return Err(rustf::Error::validation(
                "Password must be at least 8 characters long",
            ));
        }
        if password != password_confirm {
            return Err(rustf::Error::validation("Passwords do not match"));
        }
        if Users::find_by_email(&email).await?.is_some() {
            return Err(rustf::Error::validation("Email address is already registered"));
        }

        let password_hash = hash(password.as_ref(), DEFAULT_COST).map_err(|e| {
            rustf::Error::internal(format!("Failed to hash password securely: {}", e))
        })?;

        let user = Users::builder()
            .display_name(display_name.as_ref())
            .email(email.as_str())
            .password_hash(password_hash)
            .created_at(Utc::now().timestamp())
            .save()
            .await?;

        Ok(Self::to_authenticated_user(user))
    }

    pub async fn authenticate(
        form: &HashMap<String, String>,
    ) -> rustf::Result<AuthenticatedUser> {
        let email = normalize_email(required_field(form, "email")?);
        let password = required_field(form, "password")?;

        let Some(user) = Users::find_by_email(&email).await? else {
            return Err(rustf::Error::validation("Invalid email or password"));
        };

        let verified = verify(password.as_ref(), user.password_hash()).map_err(|e| {
            rustf::Error::internal(format!("Failed to verify password hash: {}", e))
        })?;

        if !verified {
            return Err(rustf::Error::validation("Invalid email or password"));
        }

        Ok(Self::to_authenticated_user(user))
    }

    fn to_authenticated_user(user: Users) -> AuthenticatedUser {
        AuthenticatedUser {
            id: user.id() as i64,
            email: user.email().to_string(),
            display_name: user.display_name().to_string(),
        }
    }
}

fn required_field<'a>(form: &'a HashMap<String, String>, key: &str) -> rustf::Result<Cow<'a, str>> {
    form.get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(Cow::from)
        .ok_or_else(|| rustf::Error::validation(format!("Field '{}' is required", key)))
}

fn normalize_email(email: Cow<'_, str>) -> String {
    email.trim().to_ascii_lowercase()
}
