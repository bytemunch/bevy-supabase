use ehttp::{Headers, Request};
use serde_json::json;

use crate::UserAttributes;

#[derive(Clone)]
pub struct Builder {
    pub url: String,
    pub headers: Headers,
}

#[derive(Clone)]
pub enum EmailOrPhone {
    Email(String),
    Phone(String),
}
// TODO detect email by looking for @
// if no @, assume is phone number

impl Builder {
    pub fn new(url: impl Into<String>) -> Builder {
        Builder {
            url: url.into(),
            headers: Headers::new(&vec![]),
        }
    }

    /// Add arbitrary headers to the request. For instance when you may want to connect
    /// through an API gateway that needs an API key header.
    pub fn insert_header(
        &mut self,
        header_name: impl ToString,
        header_value: impl ToString,
    ) -> &mut Self {
        self.headers.insert(header_name, header_value);
        self
    }

    /// Signs up for a new account
    pub fn sign_up(&self, email_or_phone: EmailOrPhone, password: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/signup", self.url);

        let body = match email_or_phone {
            EmailOrPhone::Email(email) => json!({
                "email": email,
                "password": password.as_ref(),
            }),
            EmailOrPhone::Phone(phone) => json!({
                "phone": phone,
                "password": password.as_ref()
            }),
        };

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Signs into an existing account
    pub fn sign_in(&self, email_or_phone: EmailOrPhone, password: impl AsRef<str>) -> Request {
        let query_string = String::from("?grant_type=password");

        let endpoint = format!("{}/token{}", self.url, query_string);

        let body = match email_or_phone {
            EmailOrPhone::Email(email) => json!({
                "email": email,
                "password": password.as_ref(),
            }),
            EmailOrPhone::Phone(phone) => json!({
                "phone": phone,
                "password": password.as_ref()
            }),
        };

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Sends an OTP Code and creates user if it does not exist
    pub fn send_otp(&self, email_or_phone: EmailOrPhone, should_create_user: bool) -> Request {
        let endpoint = format!("{}/otp", self.url);

        let body = match email_or_phone {
            EmailOrPhone::Email(email) => json!({
                "email": email,
                "should_create_user": should_create_user
            }),
            EmailOrPhone::Phone(phone) => json!({
                "phone": phone,
                "should_create_user": should_create_user
            }),
        };

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    pub fn verify_otp<T: serde::Serialize>(&self, params: T) -> Request {
        let endpoint = format!("{}/verify", self.url);

        let body = serde_json::to_value(&params).unwrap();

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Signs the current user out
    pub fn sign_out(&self, access_token: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/logout", self.url);

        let mut headers = self.headers.clone();
        let bearer = format!("Bearer {}", access_token.as_ref());
        headers.insert("Authorization", bearer.as_str());

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: Default::default(),
        }
    }

    /// Sends password recovery email
    pub fn reset_password_for_email(&self, email: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/recover", self.url);

        let body = json!({
            "email": email.as_ref(),
        });

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Refreshes the current session by refresh token
    pub fn refresh_access_token(&self, refresh_token: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/token?grant_type=refresh_token", self.url);
        let body = json!({ "refresh_token": refresh_token.as_ref() });

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Gets a user by access token
    pub fn get_user(&self, jwt: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/user", self.url);

        let mut headers = self.headers.clone();
        let bearer = format!("Bearer {}", jwt.as_ref());
        headers.insert("Authorization", bearer.as_str());

        Request {
            method: "GET".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: Default::default(),
        }
    }

    /// Updates a user
    pub fn update_user(&self, user: UserAttributes, jwt: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/user", self.url);

        let mut headers = self.headers.clone();
        let bearer = format!("Bearer {}", jwt.as_ref());
        headers.insert("Authorization", bearer.as_str());

        let body = json!({"email": user.email, "password": user.password, "data": user.data});

        Request {
            method: "PUT".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Invites a user via email
    pub fn invite_user_by_email(&self, email: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/invite", self.url);

        let body = json!({
            "email": email.as_ref(),
        });

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Lists all users based on a query string
    pub fn list_users(&self, query_string: Option<String>) -> Request {
        let endpoint = match query_string {
            Some(query) => format!("{}/admin/users{}", self.url, query),
            None => format!("{}/admin/users", self.url),
        };

        Request {
            method: "GET".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: Default::default(),
        }
    }

    /// Gets a user by id
    pub fn get_user_by_id(&self, user_id: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/admin/users/{}", self.url, user_id.as_ref());

        Request {
            method: "GET".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: Default::default(),
        }
    }

    /// Creates a user
    pub fn create_user<T: serde::Serialize>(&self, user: T) -> Request {
        let endpoint = format!("{}/admin/users", self.url);

        let body = serde_json::to_value(&user).unwrap();

        Request {
            method: "POST".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Updates a user by id
    pub fn update_user_by_id<T: serde::Serialize>(&self, id: impl AsRef<str>, user: T) -> Request {
        let endpoint = format!("{}/admin/users/{}", self.url, id.as_ref());

        let body = serde_json::to_value(&user).unwrap();
        Request {
            method: "PUT".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: body.to_string().into(),
        }
    }

    /// Deletes a user by id
    pub fn delete_user(&self, user_id: impl AsRef<str>) -> Request {
        let endpoint = format!("{}/admin/users/{}", self.url, user_id.as_ref());

        Request {
            method: "DELETE".to_string(),
            url: endpoint,
            headers: self.headers.clone(),
            body: Default::default(),
        }
    }
}
