use std::borrow::Borrow;
use std::fmt::{Display, Formatter};
use std::ops::Deref;

macro_rules! define_string_id {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub String);

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self(value)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                Display::fmt(self.0.as_str(), f)
            }
        }

        impl Deref for $name {
            type Target = str;
            fn deref(&self) -> &Self::Target {
                self.0.as_str()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.0.as_str()
            }
        }

        impl Borrow<str> for $name {
            fn borrow(&self) -> &str {
                self.0.as_str()
            }
        }
    };
}

define_string_id!(OrderId);
define_string_id!(ChatId);
define_string_id!(LotId);
