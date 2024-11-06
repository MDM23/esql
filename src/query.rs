use std::{fmt::Display, ops::Add};

use crate::Type;

/// Marker trait for a trusted string-like value that can be used in a SQL query
trait Trusted: ToString {}

/// Wrapper type for a string whose trustworthiness must be validated by the
/// developer. An instance of it can only be constructed by using the unsafe
/// function [trusted]. This draws the developers attention to the possible
/// risks and allows easier reviewing of code sections that potentially
/// introduce SQL-injections.
pub struct TrustedString(String);

impl ToString for TrustedString {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl Trusted for &'static str {}
impl Trusted for TrustedString {}

/// Turns the given string into a TrustedString instance. As the source is not a
/// &'static str here, we cannot be sure that the source does not contain any
/// unverified user input. In order to draw your attention to the risk of
/// SQL-injections, this function is marked as unsafe. It can be used like
/// follows:
///
/// ```
/// // Let's just imagine that field comes from an untrusted source ...
/// let field = String::from("username");
///
/// if ["id", "username", "email"].contains(&field.as_str()) {
///     // SAFETY: field passed the whitelist-check
///     let q = esql::query("SELECT") + unsafe {
///         esql::trusted(format!("{field} as my_field"))
///     };
///
///     assert_eq!(q.to_string(), "SELECT username as my_field");
/// }
/// ```
pub unsafe fn trusted(value: impl ToString) -> TrustedString {
    TrustedString(value.to_string())
}

#[derive(Debug)]
pub struct QueryBuffer<'a> {
    query: String,
    args: Vec<Type<'a>>,
}

impl<'a> QueryBuffer<'a> {
    fn push(&mut self, glue: &str, other: &mut Self) {
        self.query.push_str(glue);
        self.query.push_str(&other.query);
        self.args.append(&mut other.args);
    }
}

impl<'a, T: Trusted> From<T> for QueryBuffer<'a> {
    fn from(value: T) -> Self {
        QueryBuffer {
            query: value.to_string(),
            args: Vec::new(),
        }
    }
}

impl<'a, T, A1> Into<QueryBuffer<'a>> for (T, A1)
where
    T: Trusted,
    A1: Into<Type<'a>>,
{
    fn into(self) -> QueryBuffer<'a> {
        QueryBuffer {
            query: self.0.to_string(),
            args: vec![self.1.into()],
        }
    }
}

impl<'a, T, A1, A2> Into<QueryBuffer<'a>> for (T, A1, A2)
where
    T: Trusted,
    A1: Into<Type<'a>>,
    A2: Into<Type<'a>>,
{
    fn into(self) -> QueryBuffer<'a> {
        QueryBuffer {
            query: self.0.to_string(),
            args: vec![self.1.into(), self.2.into()],
        }
    }
}

impl<'a, T, A1, A2, A3> Into<QueryBuffer<'a>> for (T, A1, A2, A3)
where
    T: Trusted,
    A1: Into<Type<'a>>,
    A2: Into<Type<'a>>,
    A3: Into<Type<'a>>,
{
    fn into(self) -> QueryBuffer<'a> {
        QueryBuffer {
            query: self.0.to_string(),
            args: vec![self.1.into(), self.2.into(), self.3.into()],
        }
    }
}

#[derive(Debug)]
pub struct Query<'a, S> {
    buffer: QueryBuffer<'a>,
    state: S,
}

#[derive(Debug)]
pub struct Raw;

#[derive(Debug)]
pub struct Where;

#[derive(Debug)]
pub struct Having;

#[derive(Debug)]
pub struct Suffixed;

pub fn query<'a>(q: impl Into<QueryBuffer<'a>>) -> Query<'a, Raw> {
    Query {
        buffer: q.into(),
        state: Raw,
    }
}

impl<'a> Query<'a, Raw> {
    pub fn wh(mut self, q: impl Into<QueryBuffer<'a>>) -> Query<'a, Where> {
        self.buffer.push(" WHERE ", &mut q.into());

        Query {
            buffer: self.buffer,
            state: Where,
        }
    }

    pub fn having(mut self, q: impl Into<QueryBuffer<'a>>) -> Query<'a, Having> {
        self.buffer.push(" HAVING ", &mut q.into());

        Query {
            buffer: self.buffer,
            state: Having,
        }
    }
}

impl<'a> Query<'a, Where> {
    pub fn and(mut self, q: impl Into<QueryBuffer<'a>>) -> Query<'a, Where> {
        self.buffer.push(" AND ", &mut q.into());
        self
    }

    pub fn or(mut self, q: impl Into<QueryBuffer<'a>>) -> Query<'a, Where> {
        self.buffer.push(" OR ", &mut q.into());
        self
    }

    pub fn having(mut self, q: impl Into<QueryBuffer<'a>>) -> Query<'a, Having> {
        self.buffer.push(" HAVING ", &mut q.into());

        Query {
            buffer: self.buffer,
            state: Having,
        }
    }
}

impl<'a, Q> Add<Q> for Query<'a, Raw>
where
    Q: Into<QueryBuffer<'a>>,
{
    type Output = Self;

    fn add(mut self, rhs: Q) -> Self::Output {
        self.buffer.push(" ", &mut rhs.into());
        self
    }
}

impl<'a, Q> Add<Q> for Query<'a, Where>
where
    Q: Into<QueryBuffer<'a>>,
{
    type Output = Query<'a, Suffixed>;

    fn add(mut self, rhs: Q) -> Self::Output {
        self.buffer.push(" ", &mut rhs.into());

        Query {
            buffer: self.buffer,
            state: Suffixed,
        }
    }
}

impl<'a, Q> Add<Q> for Query<'a, Having>
where
    Q: Into<QueryBuffer<'a>>,
{
    type Output = Query<'a, Suffixed>;

    fn add(mut self, rhs: Q) -> Self::Output {
        self.buffer.push(" ", &mut rhs.into());

        Query {
            buffer: self.buffer,
            state: Suffixed,
        }
    }
}

impl<'a, Q> Add<Q> for Query<'a, Suffixed>
where
    Q: Into<QueryBuffer<'a>>,
{
    type Output = Self;

    fn add(mut self, rhs: Q) -> Self::Output {
        self.buffer.push(" ", &mut rhs.into());
        self
    }
}

pub struct Expr<'a>(QueryBuffer<'a>);

pub fn expr<'a>(q: impl Into<QueryBuffer<'a>>) -> Expr<'a> {
    Expr(q.into())
}

impl<'a> Expr<'a> {
    pub fn and(mut self, q: impl Into<QueryBuffer<'a>>) -> Self {
        self.0.push(" AND ", &mut q.into());
        self
    }

    pub fn or(mut self, q: impl Into<QueryBuffer<'a>>) -> Self {
        self.0.push(" OR ", &mut q.into());
        self
    }
}

impl<'a> From<Expr<'a>> for QueryBuffer<'a> {
    fn from(mut value: Expr<'a>) -> Self {
        value.0.query = String::from("(") + &value.0.query + ")";
        value.0
    }
}

pub fn in_expr<'a>(
    subject: impl Into<QueryBuffer<'a>>,
    values: impl IntoIterator<Item = impl Into<Type<'a>>>,
) -> QueryBuffer<'a> {
    let mut buffer = subject.into();
    let args: Vec<Type> = values.into_iter().map(Into::into).collect();

    if args.is_empty() {
        return QueryBuffer::from("1=0");
    }

    let mut args = QueryBuffer {
        query: String::from("(") + "?,".repeat(args.len()).trim_end_matches(',') + ")",
        args,
    };

    buffer.push(" IN ", &mut args);
    buffer
}

pub struct Fields<'a>(QueryBuffer<'a>);

pub fn fields<'a>(items: impl IntoIterator<Item = impl Into<Type<'a>>>) -> Fields<'a> {
    todo!()
}

pub enum ArgFormat {
    QuestionMark,
    Indexed,
}

impl<'a, T> Query<'a, T> {
    pub fn build(self, format: ArgFormat) -> (String, Vec<Type<'a>>) {
        if let ArgFormat::Indexed = format {
            self.build_indexed()
        } else {
            (self.buffer.query, self.buffer.args)
        }
    }

    fn build_indexed(self) -> (String, Vec<Type<'a>>) {
        let mut n = 0;

        (
            // TODO: Enhance this process and support question marks in strings
            self.buffer
                .query
                .chars()
                .map(|c| match c {
                    '?' => {
                        n = n + 1;
                        String::from("$") + &n.to_string()
                    }
                    c => c.to_string(),
                })
                .collect::<Vec<_>>()
                .concat(),
            self.buffer.args,
        )
    }
}

impl<S> Display for Query<'_, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.buffer.query)
    }
}
