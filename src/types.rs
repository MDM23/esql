use std::borrow::Cow;

macro_rules! make_args {
    (
		simple {$($target:ident($source:ty),)+}
		extra {$($extra:tt)*}
	) => {
		#[derive(Debug, PartialEq)]
        pub enum Type<'a> {
			$($target($source),)+
			$($extra)*
		}

		$(impl<'a> Into<Type<'a>> for $source {
            fn into(self) -> Type<'a> {
                Type::$target(self)
            }
        })+
    };
}

make_args! {
    simple {
        Bool(bool),
        Int8(i8),
        Int16(i16),
        Int32(i32),
        Int64(i64),
        Isize(isize),
        UInt8(u8),
        UInt16(u16),
        UInt32(u32),
        UInt64(u64),
        Usize(usize),
        Float(f32),
        Double(f64),
    }
    extra {
        Null,
        String(Cow<'a, str>),

        #[cfg(feature = "serde-json")]
        Json(serde_json::Value),

        #[cfg(feature = "time")]
        OffsetDateTime(time::OffsetDateTime),

        #[cfg(feature = "uuid")]
        Uuid(uuid::Uuid),
    }
}

impl<'a, A: Into<Type<'a>>> Into<Type<'a>> for Option<A> {
    fn into(self) -> Type<'a> {
        match self {
            Some(arg) => arg.into(),
            None => Type::Null,
        }
    }
}

impl<'a> Into<Type<'a>> for &'a str {
    fn into(self) -> Type<'a> {
        Type::String(self.into())
    }
}

impl<'a> Into<Type<'a>> for String {
    fn into(self) -> Type<'a> {
        Type::String(self.into())
    }
}

#[cfg(feature = "serde-json")]
impl<'a> Into<Type<'a>> for serde_json::Value {
    fn into(self) -> Type<'a> {
        Type::Json(self)
    }
}

#[cfg(feature = "time")]
impl<'a> Into<Type<'a>> for time::OffsetDateTime {
    fn into(self) -> Type<'a> {
        Type::OffsetDateTime(self)
    }
}

#[cfg(feature = "uuid")]
impl<'a> Into<Type<'a>> for uuid::Uuid {
    fn into(self) -> Type<'a> {
        Type::Uuid(self)
    }
}
