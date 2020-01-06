#![deny(rust_2018_idioms, warnings)]
#![deny(clippy::all, clippy::pedantic)]
#![allow(
	clippy::default_trait_access,
	clippy::let_and_return,
	clippy::let_unit_value,
	clippy::missing_errors_doc,
	clippy::module_name_repetitions,
	clippy::must_use_candidate,
	clippy::shadow_unrelated,
	clippy::similar_names,
	clippy::too_many_lines,
	clippy::unneeded_field_pattern,
	clippy::unknown_clippy_lints,
	clippy::use_self,
)]

//! This is a pure Rust implementation of a D-Bus client.
//!
//! Create a client with [`client::Client::new`]
//!
//!
//! # Example
//!
//! ## Connect to the session bus and list all names
//!
//! ```rust
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #
//! let connection =
//!     dbus_pure::conn::Connection::new(
//!         dbus_pure::conn::BusPath::Session,
//!         dbus_pure::conn::SaslAuthType::Uid,
//!     )?;
//! let mut client = dbus_pure::client::Client::new(connection)?;
//!
//! // List all names by calling the `org.freedesktop.DBus.ListNames` method
//! // on the `/org/freedesktop/DBus` object at the destination `org.freedesktop.DBus`.
//! let names = {
//!     let body =
//!         client.method_call(
//!             "org.freedesktop.DBus",
//!             dbus_pure::types::ObjectPath("/org/freedesktop/DBus".into()),
//!             "org.freedesktop.DBus",
//!             "ListNames",
//!             None,
//!         )?
//!         .ok_or("ListNames response does not have a body")?;
//!     let body: Vec<String> = serde::Deserialize::deserialize(body)?;
//!     body
//! };
//!
//! for name in names {
//!     println!("{}", name);
//! }
//! #
//! # Ok(())
//! # }
//! ```

pub mod client;

pub mod conn;

pub(crate) mod de;

pub(crate) mod ser;

pub mod std2;

pub mod types;

#[derive(Clone, Copy, Debug)]
pub enum Endianness {
	Big,
	Little,
}

macro_rules! endianness_from_bytes {
	($($fn:ident -> $ty:ty,)*) => {
		impl Endianness {
			$(
				fn $fn(self, bytes: [u8; std::mem::size_of::<$ty>()]) -> $ty {
					match self {
						Endianness::Big => <$ty>::from_be_bytes(bytes),
						Endianness::Little => <$ty>::from_le_bytes(bytes),
					}
				}
			)*
		}
	};
}

endianness_from_bytes! {
	i16_from_bytes -> i16,
	i32_from_bytes -> i32,
	i64_from_bytes -> i64,

	u16_from_bytes -> u16,
	u32_from_bytes -> u32,
	u64_from_bytes -> u64,

	f64_from_bytes -> f64,
}


macro_rules! endianness_to_bytes {
	($($fn:ident -> $ty:ty,)*) => {
		impl Endianness {
			$(
				fn $fn(self, value: $ty) -> [u8; std::mem::size_of::<$ty>()] {
					match self {
						Endianness::Big => <$ty>::to_be_bytes(value),
						Endianness::Little => <$ty>::to_le_bytes(value),
					}
				}
			)*
		}
	};
}

endianness_to_bytes! {
	i16_to_bytes -> i16,
	i32_to_bytes -> i32,
	i64_to_bytes -> i64,

	u16_to_bytes -> u16,
	u32_to_bytes -> u32,
	u64_to_bytes -> u64,

	f64_to_bytes -> f64,
}
