use std::ops::{Add, BitAnd, BitOr, Not};

use crate::types::Type;

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
///     esql::select(
///         // SAFETY: field passed the whitelist-check
///         unsafe { esql::trusted(format!("{field} as my_field")) }
///     );
/// }
/// ```
pub unsafe fn trusted(value: impl ToString) -> TrustedString {
    TrustedString(value.to_string())
}

#[derive(Debug, PartialEq)]
pub struct Args<'a>(pub Vec<Type<'a>>);

pub struct ArgString<'a> {
    raw: String,
    args: Args<'a>,
}

impl<'a> ArgString<'a> {
    fn wrapped(mut self) -> Self {
        self.raw.insert(0, '(');
        self.raw.push(')');
        self
    }
}

impl<'a, T: Trusted> From<T> for ArgString<'a> {
    fn from(value: T) -> Self {
        ArgString {
            raw: value.to_string(),
            args: Args(Vec::new()),
        }
    }
}

impl<'a, T, A1> Into<ArgString<'a>> for (T, A1)
where
    T: Trusted,
    A1: Into<Type<'a>>,
{
    fn into(self) -> ArgString<'a> {
        ArgString {
            raw: self.0.to_string(),
            args: Args(vec![self.1.into()]),
        }
    }
}

impl<'a, T, A1, A2> Into<ArgString<'a>> for (T, A1, A2)
where
    T: Trusted,
    A1: Into<Type<'a>>,
    A2: Into<Type<'a>>,
{
    fn into(self) -> ArgString<'a> {
        ArgString {
            raw: self.0.to_string(),
            args: Args(vec![self.1.into(), self.2.into()]),
        }
    }
}

impl<'a, T, A1, A2, A3> Into<ArgString<'a>> for (T, A1, A2, A3)
where
    T: Trusted,
    A1: Into<Type<'a>>,
    A2: Into<Type<'a>>,
    A3: Into<Type<'a>>,
{
    fn into(self) -> ArgString<'a> {
        ArgString {
            raw: self.0.to_string(),
            args: Args(vec![self.1.into(), self.2.into(), self.3.into()]),
        }
    }
}

impl<'a> Add for ArgString<'a> {
    type Output = Self;

    fn add(mut self, mut other: Self) -> Self {
        self.raw.push_str(&other.raw);
        self.args.0.append(&mut other.args.0);
        self
    }
}

impl<'a> Add<&str> for ArgString<'a> {
    type Output = Self;

    fn add(mut self, other: &str) -> Self {
        self.raw.push_str(&other);
        self
    }
}

pub enum LogicalOp {
    And,
    Or,
}

pub struct Where<'a> {
    op: LogicalOp,
    inner: ArgString<'a>,
}

pub struct Having<'a> {
    op: LogicalOp,
    inner: ArgString<'a>,
}

enum Fragment<'a> {
    Select(ArgString<'a>),
    From(ArgString<'a>),
    Where(Where<'a>),
    Having(Having<'a>),
    Raw(ArgString<'a>),
}

impl Fragment<'_> {
    fn is_empty(&self) -> bool {
        match self {
            Fragment::Select(i) => i,
            Fragment::From(i) => i,
            Fragment::Where(Where { inner, .. }) => inner,
            Fragment::Having(Having { inner, .. }) => inner,
            Fragment::Raw(i) => i,
        }
        .raw
        .is_empty()
    }
}

pub struct Query<'a>(Vec<Fragment<'a>>);

pub fn raw<'a>(fragment: impl Into<ArgString<'a>>) -> Query<'a> {
    Query(vec![Fragment::Raw(fragment.into())])
}

pub fn select<'a>(fragment: impl Into<ArgString<'a>>) -> Query<'a> {
    Query(vec![Fragment::Select(fragment.into())])
}

pub fn from<'a>(fragment: impl Into<ArgString<'a>>) -> Query<'a> {
    Query(vec![Fragment::From(fragment.into())])
}

pub fn r#where<'a>(fragment: impl Into<ArgString<'a>>) -> Where<'a> {
    Where {
        op: LogicalOp::And,
        inner: fragment.into(),
    }
}

pub fn wh<'a>(fragment: impl Into<ArgString<'a>>) -> Where<'a> {
    r#where(fragment)
}

pub fn r#where_in<'a>(
    column: &'static str,
    values: impl IntoIterator<Item = impl Into<Type<'a>>>,
) -> Where<'a> {
    let args: Vec<Type> = values.into_iter().map(Into::into).collect();

    if args.is_empty() {
        return r#where("1=0");
    }

    Where {
        op: LogicalOp::And,
        inner: ArgString {
            raw: column.to_string()
                + " IN ("
                + &("?,".repeat(args.len()).trim_end_matches(','))
                + ")",
            args: Args(args),
        },
    }
}

pub fn wh_in<'a>(
    column: &'static str,
    values: impl IntoIterator<Item = impl Into<Type<'a>>>,
) -> Where<'a> {
    r#where_in(column, values)
}

pub fn having<'a>(fragment: impl Into<ArgString<'a>>) -> Having<'a> {
    Having {
        op: LogicalOp::And,
        inner: fragment.into(),
    }
}

pub fn raw_in<'a>(
    fragment: &'static str,
    values: impl IntoIterator<Item = impl Into<Type<'a>>>,
) -> Query<'a> {
    let args: Vec<Type> = values.into_iter().map(Into::into).collect();

    // TODO: What should we do when `values` was empty?

    Query(vec![Fragment::Raw(ArgString {
        raw: fragment.to_string()
            + " IN ("
            + &("?,".repeat(args.len()).trim_end_matches(','))
            + ")",
        args: Args(args),
    })])
}

impl<'a> Add for Query<'a> {
    type Output = Self;

    fn add(mut self, mut other: Self) -> Self {
        self.0.append(&mut other.0);
        self
    }
}

impl<'a, T> Add<T> for Query<'a>
where
    T: Into<ArgString<'a>>,
{
    type Output = Self;

    fn add(mut self, other: T) -> Self {
        self.0.push(Fragment::Raw(other.into()));
        self
    }
}

impl<'a> Add<Where<'a>> for Query<'a> {
    type Output = Self;

    fn add(mut self, other: Where<'a>) -> Self {
        self.0.push(Fragment::Where(other));
        self
    }
}

impl<'a> Add<Having<'a>> for Query<'a> {
    type Output = Self;

    fn add(mut self, other: Having<'a>) -> Self {
        self.0.push(Fragment::Having(other));
        self
    }
}

impl<'a> Add for Where<'a> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self & other
    }
}

impl<'a> BitAnd for Where<'a> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        use LogicalOp::*;

        Where {
            op: LogicalOp::And,
            inner: match (self.op, rhs.op) {
                (And, And) => self.inner + " AND " + rhs.inner,
                (And, Or) => self.inner + " AND " + rhs.inner.wrapped(),
                (Or, And) => self.inner.wrapped() + " AND " + rhs.inner,
                (Or, Or) => self.inner.wrapped() + " AND " + rhs.inner.wrapped(),
            },
        }
    }
}

impl<'a> BitOr for Where<'a> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Where {
            op: LogicalOp::Or,
            inner: self.inner + " OR " + rhs.inner,
        }
    }
}

impl<'a> Not for Where<'a> {
    type Output = Self;

    fn not(mut self) -> Self::Output {
        self.inner.raw.insert_str(0, "NOT (");
        self.inner.raw.push(')');
        self
    }
}

impl<'a> Add for Having<'a> {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        self & other
    }
}

impl<'a> BitAnd for Having<'a> {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        use LogicalOp::*;

        Having {
            op: LogicalOp::And,
            inner: match (self.op, rhs.op) {
                (And, And) => self.inner + " AND " + rhs.inner,
                (And, Or) => self.inner + " AND " + rhs.inner.wrapped(),
                (Or, And) => self.inner.wrapped() + " AND " + rhs.inner,
                (Or, Or) => self.inner.wrapped() + " AND " + rhs.inner.wrapped(),
            },
        }
    }
}

impl<'a> BitOr for Having<'a> {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Having {
            op: LogicalOp::Or,
            inner: self.inner + " OR " + rhs.inner,
        }
    }
}

impl<'a> Not for Having<'a> {
    type Output = Self;

    fn not(mut self) -> Self::Output {
        self.inner.raw.insert_str(0, "NOT (");
        self.inner.raw.push(')');
        self
    }
}

impl<'a> Query<'a> {
    pub fn build(self) -> Result<(String, Args<'a>), crate::Error> {
        let mut buffer = String::new();
        let mut args = Vec::new();

        let mut visited_select = false;
        let mut visited_from = false;
        let mut visited_where = false;
        let mut visited_having = false;

        for frag in self.0 {
            if frag.is_empty() {
                continue;
            }

            match frag {
                Fragment::Select(mut f) => {
                    if !visited_select {
                        buffer.push_str("SELECT ");
                        visited_select = true;
                        buffer.push_str(&f.raw);
                        args.append(&mut f.args.0);
                    } else {
                        buffer.push(',');
                        buffer.push_str(&f.raw);
                        args.append(&mut f.args.0);
                    }
                }
                Fragment::From(mut f) => {
                    if !visited_from {
                        buffer.push_str(" FROM ");
                        visited_from = true;
                        buffer.push_str(&f.raw);
                        args.append(&mut f.args.0);
                    } else {
                        buffer.push(',');
                        buffer.push_str(&f.raw);
                        args.append(&mut f.args.0);
                    }
                }
                Fragment::Where(mut f) => {
                    if !visited_where {
                        buffer.push_str(" WHERE ");
                        visited_where = true;

                        match f.op {
                            LogicalOp::And => {
                                buffer.push_str(&f.inner.raw);
                                args.append(&mut f.inner.args.0);
                            }
                            LogicalOp::Or => {
                                buffer.push_str("(");
                                buffer.push_str(&f.inner.raw);
                                buffer.push(')');
                                args.append(&mut f.inner.args.0);
                            }
                        }
                    } else {
                        match f.op {
                            LogicalOp::And => {
                                buffer.push_str(" AND ");
                                buffer.push_str(&f.inner.raw);
                                args.append(&mut f.inner.args.0);
                            }
                            LogicalOp::Or => {
                                buffer.push_str(" AND (");
                                buffer.push_str(&f.inner.raw);
                                buffer.push(')');
                                args.append(&mut f.inner.args.0);
                            }
                        }
                    }
                }
                Fragment::Having(mut f) => {
                    if !visited_having {
                        buffer.push_str(" HAVING ");
                        visited_having = true;

                        match f.op {
                            LogicalOp::And => {
                                buffer.push_str(&f.inner.raw);
                                args.append(&mut f.inner.args.0);
                            }
                            LogicalOp::Or => {
                                buffer.push_str("(");
                                buffer.push_str(&f.inner.raw);
                                buffer.push(')');
                                args.append(&mut f.inner.args.0);
                            }
                        }
                    } else {
                        match f.op {
                            LogicalOp::And => {
                                buffer.push_str(" AND ");
                                buffer.push_str(&f.inner.raw);
                                args.append(&mut f.inner.args.0);
                            }
                            LogicalOp::Or => {
                                buffer.push_str(" AND (");
                                buffer.push_str(&f.inner.raw);
                                buffer.push(')');
                                args.append(&mut f.inner.args.0);
                            }
                        }
                    }
                }
                Fragment::Raw(mut f) => {
                    buffer.push(' ');
                    buffer.push_str(&f.raw);
                    args.append(&mut f.args.0);
                }
            }
        }

        Ok((buffer, Args(args)))
    }
}
