use std::{
    collections::HashMap,
    io::{ErrorKind, Read, Write},
    net::TcpListener,
};

use bevy::ecs::system::Resource;
use bevy_http_client::HttpClient;
use serde_json::json;

use crate::{Session, User, UserAttributes};

#[derive(Clone, Resource)]
pub struct Api {
    url: String,
    headers: Vec<(&'static str, &'static str)>,
}

#[derive(Clone)]
pub enum EmailOrPhone {
    Email(String),
    Phone(String),
}
// TODO detect email by looking for @
// if no @, assume is phone number

impl Api {
    /// Creates a GoTrue API client.
    pub fn new(url: impl Into<String>) -> Api {
        Api {
            url: url.into(),
            headers: Vec::new(),
        }
    }

    /// Add arbitrary headers to the request. For instance when you may want to connect
    /// through an API gateway that needs an API key header.
    pub fn insert_header(mut self, header_name: &'static str, header_value: &'static str) -> Self {
        self.headers.push((header_name, header_value));
        self
    }

    /// Signs up for a new account
    pub fn sign_up(&self, email_or_phone: EmailOrPhone, password: impl AsRef<str>) -> HttpClient {
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

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&body)
    }

    /// Signs into an existing account
    pub fn sign_in(&self, email_or_phone: EmailOrPhone, password: impl AsRef<str>) -> HttpClient {
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

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&body)
    }

    /// Signs in with a provider
    ///
    /// Appropriate URI should be presented to the user before this function is called, using
    /// `get_url_for_provider()`
    ///
    /// # Example
    ///
    /// TODO
    pub async fn provider_sign_in(&mut self) -> Result<Session, Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:6969").expect("Couldn't bind port 6969.");

        let mut params = HashMap::new();

        loop {
            let (mut stream, _) = listener.accept().expect("Listener IO error");

            // This javascript is mental, I have to make fetch happen because GoTrue puts the
            // access token in the URI hash? Like is that intentional, surely should be on search
            // params. This fix does require JS in browser but most oAuth sign in pages probably do too, so
            // should be a non-issue.
            let message = String::from(
                "<script>fetch(`http://localhost:6969/token?${window.location.hash.replace('#','')})`)</script><h1>App Name Here</h1><h2>Login session sent to app.</h2><h3>You may close this tab.</h3>",
            );

            // TODO optional redirect to user provided URI

            let res = format!(
                "HTTP/1.1 200 OK\r\ncontent-length: {}\r\n\r\n{}",
                message.len(),
                message
            );

            loop {
                match stream.write(res.as_bytes()) {
                    Ok(_n) => break,
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                    Err(e) => println!("Couldn't respond. {}", e),
                }
            }

            let mut buf = [0; 4096];

            loop {
                match stream.read(&mut buf) {
                    Ok(0) => break,
                    Ok(_n) => break,
                    Err(ref e) if e.kind() == ErrorKind::WouldBlock => continue,
                    Err(e) => {
                        return Err(e.into());
                    }
                }
            }

            let raw = String::from_utf8(buf.to_vec()).unwrap();

            let request_line = raw.lines().collect::<Vec<_>>()[0];

            if !request_line.starts_with("GET /token?") {
                // If this request isn't the one we sent with JS fetch, ignore it and wait for the
                // right one.
                continue;
            }

            let split_req = request_line
                .strip_prefix("GET /token?")
                .unwrap()
                .split('&')
                .collect::<Vec<&str>>();

            for param in split_req {
                let split_param = param.split('=').collect::<Vec<&str>>();
                params.insert(split_param[0].to_owned(), split_param[1].to_owned());
            }

            if params.get("access_token").is_some() {
                break;
            }
        }

        let access_token = params.get("access_token").unwrap().clone();
        let refresh_token = params.get("refresh_token").unwrap().clone();

        let sesh = Session {
            access_token,
            refresh_token,
            token_type: "JWT".into(),
            expires_in: 3600, // TODO get correct time from params
            user: User::default(),
        };

        Ok(sesh)
    }

    /// Sends an OTP Code and creates user if it does not exist
    pub fn send_otp(&self, email_or_phone: EmailOrPhone, should_create_user: bool) -> HttpClient {
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

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&body)
    }

    pub fn verify_otp<T: serde::Serialize>(&self, params: T) -> HttpClient {
        let endpoint = format!("{}/verify", self.url);

        let body = serde_json::to_value(&params).unwrap();

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&body)
    }

    /// Signs the current user out
    pub fn sign_out(&self, access_token: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/logout", self.url);

        let mut headers = self.headers.clone();
        let bearer = format!("Bearer {}", access_token.as_ref());
        headers.push(("Authorization", bearer.as_str()));

        HttpClient::new().post(endpoint).headers(headers.as_slice())
    }

    /// Sends password recovery email
    pub fn reset_password_for_email(&self, email: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/recover", self.url);

        let body = json!({
            "email": email.as_ref(),
        });

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&body)
    }

    /// Returns a URL for provider oauth flow
    pub fn get_url_for_provider(&self, provider: &str) -> String {
        format!("{}/authorize?provider={}", self.url, provider)
    }

    /// Refreshes the current session by refresh token
    pub fn refresh_access_token(&self, refresh_token: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/token?grant_type=refresh_token", self.url);
        let body = json!({ "refresh_token": refresh_token.as_ref() });

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&body)
    }

    /// Gets a user by access token
    pub fn get_user(&self, jwt: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/user", self.url);

        let mut headers = self.headers.clone();
        let bearer = format!("Bearer {}", jwt.as_ref());
        headers.push(("Authorization", bearer.as_str()));

        HttpClient::new().get(endpoint).headers(headers.as_slice())
    }

    /// Updates a user
    pub fn update_user(&self, user: UserAttributes, jwt: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/user", self.url);

        let mut headers = self.headers.clone();
        let bearer = format!("Bearer {}", jwt.as_ref());
        headers.push(("Authorization", bearer.as_str()));

        let body = json!({"email": user.email, "password": user.password, "data": user.data});

        HttpClient::new()
            .put(endpoint)
            .headers(headers.as_slice())
            .json(&body)
    }

    /// Invites a user via email
    pub fn invite_user_by_email(&self, email: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/invite", self.url);

        let body = json!({
            "email": email.as_ref(),
        });

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&body)
    }

    /// Lists all users based on a query string
    pub fn list_users(&self, query_string: Option<String>) -> HttpClient {
        let endpoint = match query_string {
            Some(query) => format!("{}/admin/users{}", self.url, query),
            None => format!("{}/admin/users", self.url),
        };

        HttpClient::new()
            .get(endpoint)
            .headers(self.headers.as_slice())
    }

    /// Gets a user by id
    pub fn get_user_by_id(&self, user_id: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/admin/users/{}", self.url, user_id.as_ref());

        HttpClient::new()
            .get(endpoint)
            .headers(self.headers.as_slice())
    }

    /// Creates a user
    pub fn create_user<T: serde::Serialize>(&self, user: T) -> HttpClient {
        let endpoint = format!("{}/admin/users", self.url);

        let json = serde_json::to_value(&user).unwrap();

        HttpClient::new()
            .post(endpoint)
            .headers(self.headers.as_slice())
            .json(&json)
    }

    /// Updates a user by id
    pub fn update_user_by_id<T: serde::Serialize>(
        &self,
        id: impl AsRef<str>,
        user: T,
    ) -> HttpClient {
        let endpoint = format!("{}/admin/users/{}", self.url, id.as_ref());

        let json = serde_json::to_value(&user).unwrap();
        HttpClient::new()
            .put(endpoint)
            .headers(self.headers.as_slice())
            .json(&json)
    }

    /// Deletes a user by id
    pub fn delete_user(&self, user_id: impl AsRef<str>) -> HttpClient {
        let endpoint = format!("{}/admin/users/{}", self.url, user_id.as_ref());
        HttpClient::new()
            .delete(endpoint)
            .headers(self.headers.as_slice())
    }
}
