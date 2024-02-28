use ehttp::Headers;
use serde::Serialize;

#[allow(dead_code)]
#[derive(Serialize, Clone)]
enum Method {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
    HEAD,
}

impl Method {
    pub fn to_str(&self) -> &str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::PATCH => "PATCH",
            Method::DELETE => "DELETE",
            Method::HEAD => "HEAD",
        }
    }

    pub fn to_string(&self) -> String {
        self.to_str().to_string()
    }
}

/// QueryBuilder struct
#[derive(Clone)]
pub struct Builder {
    method: Method,
    url: String,
    schema: Option<String>,
    // Need this to allow access from `filter.rs`
    pub(crate) queries: Vec<(String, String)>,
    headers: Headers,
    body: Option<String>,
    is_rpc: bool,
}

// TODO: Test Unicode support
impl Builder {
    /// Creates a new `Builder` with the specified `schema`.
    pub fn new<T>(url: T, schema: Option<String>, headers: Headers) -> Self
    where
        T: Into<String>,
    {
        let mut builder = Builder {
            method: Method::GET,
            url: url.into(),
            schema,
            queries: Vec::new(),
            headers,
            body: None,
            is_rpc: false,
        };
        builder.headers.insert("Accept", "application/json");
        builder
    }

    /// Authenticates the request with JWT.
    pub fn auth<T>(mut self, token: T) -> Self
    where
        T: AsRef<str>,
    {
        self.headers
            .insert("Authorization", format!("Bearer {}", token.as_ref()));
        self
    }

    /// Performs horizontal filtering with SELECT.
    ///
    /// # Note
    ///
    /// `columns` is whitespace-sensitive, so you need to omit them unless your
    /// column name contains whitespaces.
    ///
    /// # Example
    ///
    /// Simple example:
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// let resp = client
    ///     .from("table")
    ///     .select("*")
    ///     .execute()
    /// ```
    ///
    /// Renaming columns:
    ///
    /// ```
    /// let resp = client
    ///     .from("users")
    ///     .select("name:very_very_long_column_name")
    ///     .execute()
    /// ```
    ///
    /// Casting columns:
    ///
    /// ```
    /// let resp = client
    ///     .from("users")
    ///     .select("age::text")
    ///     .execute()
    /// ```
    ///
    /// SELECTing JSON fields:
    ///
    /// ```
    /// let resp = client
    ///     .from("users")
    ///     .select("id,json_data->phones->0->>number")
    ///     .execute()
    /// ```
    ///
    /// Embedded filters (assume there is a foreign key constraint between
    /// tables `users` and `tweets`):
    ///
    /// ```
    /// let resp = client
    ///     .from("users")
    ///     .select("*,tweets(*)")
    ///     .execute()
    /// ```
    pub fn select<T>(mut self, columns: T) -> Self
    where
        T: Into<String>,
    {
        self.queries.push(("select".to_string(), columns.into()));
        self
    }

    /// Orders the result with the specified `columns`.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .select("*")
    ///     .order("username.desc.nullsfirst,age_range");
    /// ```
    pub fn order<T>(mut self, columns: T) -> Self
    where
        T: Into<String>,
    {
        self.queries.push(("order".to_string(), columns.into()));
        self
    }

    /// Orders the result of a foreign table with the specified `columns`.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("countries")
    ///     .select("name, cities(name)")
    ///     .order_with_options("name", Some("cities"), true, false);
    /// ```
    pub fn order_with_options<T, U>(
        mut self,
        columns: T,
        foreign_table: Option<U>,
        ascending: bool,
        nulls_first: bool,
    ) -> Self
    where
        T: Into<String>,
        U: Into<String>,
    {
        let mut key = "order".to_string();
        if let Some(foreign_table) = foreign_table {
            let foreign_table = foreign_table.into();
            if !foreign_table.is_empty() {
                key = format!("{}.order", foreign_table);
            }
        }

        let mut ascending_string = "desc";
        if ascending {
            ascending_string = "asc";
        }

        let mut nulls_first_string = "nullslast";
        if nulls_first {
            nulls_first_string = "nullsfirst";
        }

        let existing_order = self.queries.iter().find(|(k, _)| k == &key);
        match existing_order {
            Some((_, v)) => {
                let new_order = format!(
                    "{},{}.{}.{}",
                    v,
                    columns.into(),
                    ascending_string,
                    nulls_first_string
                );
                self.queries.push((key, new_order));
            }
            None => {
                self.queries.push((
                    key,
                    format!(
                        "{}.{}.{}",
                        columns.into(),
                        ascending_string,
                        nulls_first_string
                    ),
                ));
            }
        }
        self
    }

    /// Limits the result with the specified `count`.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .select("*")
    ///     .limit(20);
    /// ```
    pub fn limit(mut self, count: usize) -> Self {
        self.headers.insert("Range-Unit", "items");
        self.headers.insert("Range", format!("0-{}", count - 1));
        self
    }

    /// Limits the result of a foreign table with the specified `count`.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("countries")
    ///     .select("name, cities(name)")
    ///     .foreign_table_limit(1, "cities");
    /// ```
    pub fn foreign_table_limit<T>(mut self, count: usize, foreign_table: T) -> Self
    where
        T: Into<String>,
    {
        self.queries
            .push((format!("{}.limit", foreign_table.into()), count.to_string()));
        self
    }

    /// Limits the result to rows within the specified range, inclusive.
    ///
    /// # Example
    ///
    /// This retrieves the 2nd to 5th entries in the result:
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .select("*")
    ///     .range(1, 4);
    /// ```
    pub fn range(mut self, low: usize, high: usize) -> Self {
        self.headers.insert("Range-Unit", "items");
        self.headers.insert("Range", format!("{}-{}", low, high));
        self
    }

    fn count(mut self, method: &str) -> Self {
        self.headers.insert("Range-Unit", "items");
        // Value is irrelevant, we just want the size
        self.headers.insert("Range", "0-0");
        self.headers.insert("Prefer", format!("count={}", method));
        self
    }

    /// Retrieves the (accurate) total size of the result.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .select("*")
    ///     .exact_count();
    /// ```
    pub fn exact_count(self) -> Self {
        self.count("exact")
    }

    /// Estimates the total size of the result using PostgreSQL statistics. This
    /// is faster than using `exact_count()`.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .select("*")
    ///     .planned_count();
    /// ```
    pub fn planned_count(self) -> Self {
        self.count("planned")
    }

    /// Retrieves the total size of the result using some heuristics:
    /// `exact_count` for smaller sizes, `planned_count` for larger sizes.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .select("*")
    ///     .estimated_count();
    /// ```
    pub fn estimated_count(self) -> Self {
        self.count("estimated")
    }

    /// Retrieves only one row from the result.
    ///
    /// # Example
    ///
    /// ```
    /// use postgrest::Postgrest;
    ///
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .select("*")
    ///     .single();
    /// ```
    pub fn single(mut self) -> Self {
        self.headers
            .insert("Accept", "application/vnd.pgrst.object+json");
        self
    }

    /// Performs an INSERT of the `body` (in JSON) into the table.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .insert(r#"[{ "username": "soedirgo", "status": "online" },
    ///                 { "username": "jose", "status": "offline" }]"#);
    /// ```
    pub fn insert<T>(mut self, body: T) -> Self
    where
        T: Into<String>,
    {
        self.method = Method::POST;
        self.headers.insert("Prefer", "return=representation");
        self.body = Some(body.into());
        self
    }

    /// Performs an upsert of the `body` (in JSON) into the table.
    ///
    /// # Note
    ///
    /// This merges duplicates by default. Ignoring duplicates is possible via
    /// PostgREST, but is currently unsupported.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .upsert(r#"[{ "username": "soedirgo", "status": "online" },
    ///                 { "username": "jose", "status": "offline" }]"#);
    /// ```
    pub fn upsert<T>(mut self, body: T) -> Self
    where
        T: Into<String>,
    {
        self.method = Method::POST;
        self.headers.insert(
            "Prefer",
            "return=representation,resolution=merge-duplicates",
        );
        self.body = Some(body.into());
        self
    }

    /// Resolve upsert conflicts on unique columns other than the primary key.
    ///
    /// # Note
    ///
    /// This informs PostgREST to resolve upsert conflicts through an
    /// alternative, unique index other than the primary key of the table.
    /// See the related
    /// [PostgREST documentation](https://postgrest.org/en/stable/api.html?highlight=upsert#on-conflict).
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// // Suppose `users` are keyed an SERIAL primary key,
    /// // but have a unique index on `username`.
    /// client
    ///     .from("users")
    ///     .upsert(r#"[{ "username": "soedirgo", "status": "online" },
    ///                 { "username": "jose", "status": "offline" }]"#)
    ///     .on_conflict("username");
    /// ```
    pub fn on_conflict<T>(mut self, columns: T) -> Self
    where
        T: Into<String>,
    {
        self.queries
            .push(("on_conflict".to_string(), columns.into()));
        self
    }

    /// Performs an UPDATE using the `body` (in JSON) on the table.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .eq("username", "soedirgo")
    ///     .update(r#"{ "status": "offline" }"#);
    /// ```
    pub fn update<T>(mut self, body: T) -> Self
    where
        T: Into<String>,
    {
        self.method = Method::PATCH;
        self.headers.insert("Prefer", "return=representation");
        self.body = Some(body.into());
        self
    }

    /// Performs a DELETE on the table.
    ///
    /// # Example
    ///
    /// ```
    /// let client = Postgrest::new("https://your.postgrest.endpoint");
    /// client
    ///     .from("users")
    ///     .eq("username", "soedirgo")
    ///     .delete();
    /// ```
    pub fn delete(mut self) -> Self {
        self.method = Method::DELETE;
        self.headers.insert("Prefer", "return=representation");
        self
    }

    /// Performs a stored procedure call. This should only be used through the
    /// `rpc()` method in `Postgrest`.
    pub fn rpc<T>(mut self, params: T) -> Self
    where
        T: Into<String>,
    {
        self.method = Method::POST;
        self.body = Some(params.into());
        self.is_rpc = true;
        self
    }

    /// Build the PostgREST request.
    pub fn build(mut self) -> ehttp::Request {
        if let Some(schema) = self.schema {
            let key = match self.method {
                Method::GET | Method::HEAD => "Accept-Profile",
                _ => "Content-Profile",
            };
            self.headers.insert(key, schema);
        }
        match self.method {
            Method::GET | Method::HEAD => {}
            _ => {
                self.headers.insert("Content-Type", "application/json");
            }
        };

        let mut url: url::Url = self.url.parse().expect("Malformed URI");

        for (name, value) in self.queries {
            url.query_pairs_mut()
                .append_pair(name.as_str(), value.as_str());
        }

        ehttp::Request {
            method: self.method.to_string(),
            url: url.to_string(),
            body: self.body.unwrap_or_default().into(),
            headers: self.headers,
        }
    }

    /// Executes the PostgREST request.
    pub fn execute(self) -> Result<ehttp::Response, String> {
        let req = self.build();
        ehttp::fetch_blocking(&req)
    }
}
