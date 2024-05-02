use bevy::ecs::system::Resource;
use ehttp::Headers;

use crate::builder::Builder;

#[derive(Clone, Resource)]
pub struct Client {
    url: String,
    schema: Option<String>,
    headers: Headers,
}

impl Client {
    /// Creates a Postgrest client.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("http://your.postgrest.endpoint");
    /// ```
    pub fn new<T>(url: T) -> Self
    where
        T: Into<String>,
    {
        Client {
            url: url.into(),
            schema: None,
            headers: Headers::new(&vec![]),
        }
    }

    /// Switches the schema.
    ///
    /// # Note
    ///
    /// You can only switch schemas before you call `from` or `rpc`.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("http://your.postgrest.endpoint");
    /// client.schema("private");
    /// ```
    pub fn schema<T>(&mut self, schema: T) -> &mut Self
    where
        T: Into<String>,
    {
        self.schema = Some(schema.into());
        self
    }

    /// Add arbitrary headers to the request. For instance when you may want to connect
    /// through an API gateway that needs an API key header.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint")
    ///     .insert_header("apikey", "super.secret.key")
    ///     .from("table");
    /// ```
    pub fn insert_header(
        &mut self,
        header_name: impl ToString,
        header_value: impl ToString,
    ) -> &mut Self {
        // TODO be safer with CSV / single value headers
        self.headers.insert(header_name, header_value);
        self
    }

    /// Perform a table operation.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("http://your.postgrest.endpoint");
    /// client.from("table");
    /// ```
    pub fn from<T>(&self, table: T) -> Builder
    where
        T: AsRef<str>,
    {
        let url = format!("{}/{}", self.url, table.as_ref());
        Builder::new(url, self.schema.clone(), self.headers.clone())
    }

    /// Perform a stored procedure call.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("http://your.postgrest.endpoint");
    /// client.rpc("multiply", r#"{"a": 1, "b": 2}"#);
    /// ```
    pub fn rpc<T, U>(&self, function: T, params: U) -> Builder
    where
        T: AsRef<str>,
        U: Into<String>,
    {
        let url = format!("{}/rpc/{}", self.url, function.as_ref());
        Builder::new(url, self.schema.clone(), self.headers.clone()).rpc(params)
    }
}
