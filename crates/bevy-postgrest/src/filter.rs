use crate::builder::Builder;

impl Builder {
    /// Finds all rows which doesn't satisfy the filter.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .not("eq", "name", "New Zealand")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn not<T, U, V>(mut self, operator: T, column: U, filter: V) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
        V: AsRef<str>,
    {
        self.queries.push((
            column.as_ref().into(),
            format!("not.{}.{}", operator.as_ref(), filter.as_ref()),
        ));
        self
    }

    /// Finds all rows satisfying all of the filters.
    ///
    /// # Note
    ///
    /// If your column/filter contains PostgREST's reserved characters, you need
    /// to surround them with double quotes (not percent encoded). See
    /// [here](https://postgrest.org/en/v7.0.0/api.html#reserved-characters) for
    /// details.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .and("name.eq.New Zealand,or(id.gte.1,capital.is.null)")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn and<T>(mut self, filters: T) -> Self
    where
        T: AsRef<str>,
    {
        self.queries
            .push(("and".to_string(), format!("({})", filters.as_ref())));
        self
    }

    /// Finds all rows satisfying at least one of the filters.
    ///
    /// # Note
    ///
    /// If your column/filter contains PostgREST's reserved characters, you need
    /// to surround them with double quotes (not percent encoded). See
    /// [here](https://postgrest.org/en/v7.0.0/api.html#reserved-characters) for
    /// details.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .or("name.eq.New Zealand,or(id.gte.1,capital.is.null)")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn or<T>(mut self, filters: T) -> Self
    where
        T: AsRef<str>,
    {
        self.queries
            .push(("or".to_string(), format!("({})", filters.as_ref())));
        self
    }

    /// Finds all rows whose value on the stated `column` exactly matches the
    /// specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .eq("name", "New Zealand")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn eq<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("eq.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose value on the stated `column` doesn't match the
    /// specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .neq("name", "China")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn neq<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("neq.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose value on the stated `column` is greater than the
    /// specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .gt("id", "20")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn gt<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("gt.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose value on the stated `column` is greater than or
    /// equal to the specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .gte("id", "20")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn gte<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("gte.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose value on the stated `column` is less than the
    /// specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .lt("id", "20")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn lt<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("lt.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose value on the stated `column` is less than or equal
    /// to the specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .lte("id", "20")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn lte<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("lte.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose value in the stated `column` matches the supplied
    /// `pattern` (case sensitive).
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .like("name", "%United%")
    ///     .select("*")
    ///     .execute()
    ///
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .like("name", "%United States%")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn like<T, U>(mut self, column: T, pattern: U) -> Self
    where
        T: AsRef<str>,
        U: Into<String>,
    {
        let pattern = pattern.into().replace('%', "*");
        self.queries
            .push((column.as_ref().into(), format!("like.{}", pattern)));
        self
    }

    /// Finds all rows whose value in the stated `column` matches the supplied
    /// `pattern` (case insensitive).
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .ilike("name", "%United%")
    ///     .select("*")
    ///     .execute()
    ///
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .ilike("name", "%united states%")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn ilike<T, U>(mut self, column: T, pattern: U) -> Self
    where
        T: AsRef<str>,
        U: Into<String>,
    {
        let pattern = pattern.into().replace('%', "*");
        self.queries
            .push((column.as_ref().into(), format!("ilike.{}", pattern)));
        self
    }

    /// A check for exact equality (null, true, false), finds all rows whose
    /// value on the stated `column` exactly match the specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .is("name", "null")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn is<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("is.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose value on the stated `column` is found on the
    /// specified `values`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .in_("name", vec!["China", "France"])
    ///     .select("*")
    ///     .execute()
    ///
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("countries")
    ///     .in_("capitals", vec!["Beijing,China", "Paris,France"])
    ///     .select("*")
    ///     .execute()
    ///
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("recipes")
    ///     .in_("food_supplies", vec!["carrot (big)", "carrot (small)"])
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn in_<T, U, V>(mut self, column: T, values: U) -> Self
    where
        T: AsRef<str>,
        U: IntoIterator<Item = V>,
        V: AsRef<str>,
    {
        let mut values: String = values
            .into_iter()
            .fold(String::new(), |a, s| a + s.as_ref() + ",");
        values.pop();
        self.queries
            .push((column.as_ref().into(), format!("in.({})", values)));
        self
    }

    /// Finds all rows whose json, array, or range value on the stated `column`
    /// contains the values specified in `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("users")
    ///     .cs("age_range", "(10,20)")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn cs<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: AsRef<str>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.as_ref().into(), format!("cs.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose json, array, or range value on the stated `column`
    /// is contained by the specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("users")
    ///     .cd("age_range", "(10,20)")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn cd<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: Into<String>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.into(), format!("cd.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose range value on the stated `column` is strictly to
    /// the left of the specified `range`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("users")
    ///     .sl("age_range", (10, 20))
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn sl<T>(mut self, column: T, range: (i64, i64)) -> Self
    where
        T: Into<String>,
    {
        self.queries
            .push((column.into(), format!("sl.({},{})", range.0, range.1)));
        self
    }

    /// Finds all rows whose range value on the stated `column` is strictly to
    /// the right of the specified `range`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .sr("age_range", (10, 20))
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn sr<T>(mut self, column: T, range: (i64, i64)) -> Self
    where
        T: Into<String>,
    {
        self.queries
            .push((column.into(), format!("sr.({},{})", range.0, range.1)));
        self
    }

    /// Finds all rows whose range value on the stated `column` does not extend
    /// to the left of the specified `range`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .nxl("age_range", (10, 20))
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn nxl<T>(mut self, column: T, range: (i64, i64)) -> Self
    where
        T: Into<String>,
    {
        self.queries
            .push((column.into(), format!("nxl.({},{})", range.0, range.1)));
        self
    }

    /// Finds all rows whose range value on the stated `column` does not extend
    /// to the right of the specified `range`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .nxr("age_range", (10, 20))
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn nxr<T>(mut self, column: T, range: (i64, i64)) -> Self
    where
        T: Into<String>,
    {
        self.queries
            .push((column.into(), format!("nxr.({},{})", range.0, range.1)));
        self
    }

    /// Finds all rows whose range value on the stated `column` is adjacent to
    /// the specified `range`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .adj("age_range", (10, 20))
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn adj<T>(mut self, column: T, range: (i64, i64)) -> Self
    where
        T: Into<String>,
    {
        self.queries
            .push((column.into(), format!("adj.({},{})", range.0, range.1)));
        self
    }

    /// Finds all rows whose array or range value on the stated `column`
    /// overlaps with the specified `filter`.
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .ov("age_range", "(10,20)")
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn ov<T, U>(mut self, column: T, filter: U) -> Self
    where
        T: Into<String>,
        U: AsRef<str>,
    {
        self.queries
            .push((column.into(), format!("ov.{}", filter.as_ref())));
        self
    }

    /// Finds all rows whose tsvector value on the stated `column` matches
    /// to_tsquery(`tsquery`).
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .fts("phrase", "The Fat Cats", Some("english"))
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn fts<T, U>(mut self, column: T, tsquery: U, config: Option<&str>) -> Self
    where
        T: Into<String>,
        U: AsRef<str>,
    {
        let config = if let Some(conf) = config {
            format!("({})", conf)
        } else {
            String::new()
        };
        self.queries
            .push((column.into(), format!("fts{}.{}", config, tsquery.as_ref())));
        self
    }

    /// Finds all rows whose tsvector value on the stated `column` matches
    /// plainto_tsquery(`tsquery`).
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .plfts("phrase", "The Fat Cats", None)
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn plfts<T, U>(mut self, column: T, tsquery: U, config: Option<&str>) -> Self
    where
        T: Into<String>,
        U: AsRef<str>,
    {
        let config = if let Some(conf) = config {
            format!("({})", conf)
        } else {
            String::new()
        };
        self.queries.push((
            column.into(),
            format!("plfts{}.{}", config, tsquery.as_ref()),
        ));
        self
    }

    /// Finds all rows whose tsvector value on the stated `column` matches
    /// phraseto_tsquery(`tsquery`).
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .phfts("phrase", "The Fat Cats", Some("english"))
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn phfts<T, U>(mut self, column: T, tsquery: U, config: Option<&str>) -> Self
    where
        T: Into<String>,
        U: AsRef<str>,
    {
        let config = if let Some(conf) = config {
            format!("({})", conf)
        } else {
            String::new()
        };
        self.queries.push((
            column.into(),
            format!("phfts{}.{}", config, tsquery.as_ref()),
        ));
        self
    }

    /// Finds all rows whose tsvector value on the stated `column` matches
    /// websearch_to_tsquery(`tsquery`).
    ///
    /// # Example
    ///
    /// ```
    /// let resp = Postgrest::new("http://localhost:3000")
    ///     .from("table")
    ///     .wfts("phrase", "The Fat Cats", None)
    ///     .select("*")
    ///     .execute()
    /// ```
    pub fn wfts<T, U>(mut self, column: T, tsquery: U, config: Option<&str>) -> Self
    where
        T: Into<String>,
        U: AsRef<str>,
    {
        let config = if let Some(conf) = config {
            format!("({})", conf)
        } else {
            String::new()
        };
        self.queries.push((
            column.into(),
            format!("wfts{}.{}", config, tsquery.as_ref()),
        ));
        self
    }
}
