#[derive(Debug)]
pub(crate) struct Serializer<'ser> {
	buf: &'ser mut Vec<u8>,
	start: usize,
	endianness: crate::Endianness,
}

impl<'ser> Serializer<'ser> {
	pub(crate) fn new(buf: &'ser mut Vec<u8>, endianness: crate::Endianness) -> Self {
		let start = buf.len();
		Serializer {
			endianness,
			buf,
			start,
		}
	}

	pub(crate) fn pad_to(&mut self, alignment: usize) {
		let pos = self.buf.len() - self.start;
		let new_pos = ((pos + alignment - 1) / alignment) * alignment;
		let new_len = self.start + new_pos;
		self.buf.resize(new_len, 0);
	}
}

impl<'ser, 'a> serde::Serializer for &'a mut Serializer<'ser> {
	type Ok = ();
	type Error = SerializeError;
	type SerializeSeq = SeqSerializer<'ser, 'a>;
	type SerializeTuple = TupleSerializer<'ser, 'a>;
	type SerializeTupleStruct = TupleStructSerializer;
	type SerializeTupleVariant = TupleVariantSerializer;
	type SerializeMap = MapSerializer;
	type SerializeStruct = StructSerializer<'ser, 'a>;
	type SerializeStructVariant = StructVariantSerializer;

	fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
		let v: u32 = if v { 1 } else { 0 };
		self.serialize_u32(v)
	}

	fn serialize_i8(self, _v: i8) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
		self.pad_to(2);
		self.buf.extend_from_slice(&self.endianness.i16_to_bytes(v));
		Ok(())
	}

	fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
		self.pad_to(4);
		self.buf.extend_from_slice(&self.endianness.i32_to_bytes(v));
		Ok(())
	}

	fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
		self.pad_to(8);
		self.buf.extend_from_slice(&self.endianness.i64_to_bytes(v));
		Ok(())
	}

	fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
		self.buf.push(v);
		Ok(())
	}

	fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
		self.pad_to(2);
		self.buf.extend_from_slice(&self.endianness.u16_to_bytes(v));
		Ok(())
	}

	fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
		self.pad_to(4);
		self.buf.extend_from_slice(&self.endianness.u32_to_bytes(v));
		Ok(())
	}

	fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
		self.pad_to(8);
		self.buf.extend_from_slice(&self.endianness.u64_to_bytes(v));
		Ok(())
	}

	fn serialize_f32(self, _v: f32) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
		self.pad_to(8);
		self.buf.extend_from_slice(&self.endianness.f64_to_bytes(v));
		Ok(())
	}

	fn serialize_char(self, _v: char) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
		serde::Serialize::serialize(&crate::Slice { inner: v.as_bytes(), alignment: 1 }, &mut *self)?;
		serde::Serialize::serialize(&b'\0', &mut *self)?;
		Ok(())
	}

	fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok, Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}

	fn serialize_newtype_struct<T>(self, _name: &'static str, _value: &T) -> Result<Self::Ok, Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn serialize_newtype_variant<T>(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _value: &T) -> Result<Self::Ok, Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
		// D-Bus requires pre-element padding even if the array is empty. So serialize_seq needs to know the alignment of the type that it's going to be used for,
		// even if no value of that type gets serialized.
		//
		// Since serde does not have any way to transmit that information from the type's Serialize impl to here, we abuse the len parameter
		// to get that information instead.
		//
		// Nothing outside this crate uses this serializer, so this hack should be okay.
		let alignment = match len {
			Some(alignment @ 1) |
			Some(alignment @ 2) |
			Some(alignment @ 4) |
			Some(alignment @ 8) => alignment,
			alignment => panic!("unexpected alignment {:?}", alignment),
		};

		serde::Serialize::serialize(&0_u32, &mut *self)?;
		let data_len_pos = self.buf.len() - 4;

		self.pad_to(alignment);

		let data_start_pos = self.buf.len();

		Ok(SeqSerializer {
			inner: self,
			data_len_pos,
			data_start_pos,
		})
	}

	fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
		Ok(TupleSerializer(self))
	}

	fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
		unimplemented!();
	}

	fn serialize_tuple_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeTupleVariant, Self::Error> {
		unimplemented!();
	}

	fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
		unimplemented!();
	}

	fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
		self.pad_to(8);
		Ok(StructSerializer(self))
	}

	fn serialize_struct_variant(self, _name: &'static str, _variant_index: u32, _variant: &'static str, _len: usize) -> Result<Self::SerializeStructVariant, Self::Error> {
		unimplemented!();
	}

	fn is_human_readable(&self) -> bool {
		false
	}
}

pub(crate) struct SeqSerializer<'ser, 'a> {
	inner: &'a mut Serializer<'ser>,
	data_len_pos: usize,
	data_start_pos: usize,
}

impl<'ser, 'a> serde::ser::SerializeSeq for SeqSerializer<'ser, 'a> {
	type Ok = ();
	type Error = SerializeError;

	fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		value.serialize(&mut *self.inner)?;
		Ok(())
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		let data_end_pos = self.inner.buf.len();

		let data_len: u32 = std::convert::TryInto::try_into(data_end_pos - self.data_start_pos).map_err(serde::ser::Error::custom)?;

		self.inner.buf[self.data_len_pos..(self.data_len_pos + 4)].copy_from_slice(&self.inner.endianness.u32_to_bytes(data_len));

		Ok(())
	}
}

pub(crate) struct TupleSerializer<'ser, 'a>(&'a mut Serializer<'ser>);

impl<'ser, 'a> serde::ser::SerializeTuple for TupleSerializer<'ser, 'a> {
	type Ok = ();
	type Error = SerializeError;

	fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		value.serialize(&mut *self.0)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

pub(crate) struct TupleStructSerializer;

impl serde::ser::SerializeTupleStruct for TupleStructSerializer {
	type Ok = ();
	type Error = SerializeError;

	fn serialize_field<T>(&mut self, _value: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}
}

pub(crate) struct TupleVariantSerializer;

impl serde::ser::SerializeTupleVariant for TupleVariantSerializer {
	type Ok = ();
	type Error = SerializeError;

	fn serialize_field<T>(&mut self, _value: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}
}

pub(crate) struct MapSerializer;

impl serde::ser::SerializeMap for MapSerializer {
	type Ok = ();
	type Error = SerializeError;

	fn serialize_key<T>(&mut self, _key: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn serialize_value<T>(&mut self, _value: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}
}

pub(crate) struct StructSerializer<'ser, 'a>(&'a mut Serializer<'ser>);

impl<'ser, 'a> serde::ser::SerializeStruct for StructSerializer<'ser, 'a> {
	type Ok = ();
	type Error = SerializeError;

	fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		value.serialize(&mut *self.0)
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		Ok(())
	}
}

pub(crate) struct StructVariantSerializer;

impl serde::ser::SerializeStructVariant for StructVariantSerializer {
	type Ok = ();
	type Error = SerializeError;

	fn serialize_field<T>(&mut self, _key: &'static str, _value: &T) -> Result<(), Self::Error> where T: serde::Serialize + ?Sized {
		unimplemented!();
	}

	fn end(self) -> Result<Self::Ok, Self::Error> {
		unimplemented!();
	}
}

/// An error from serializing a value using the D-Bus binary protocol.
#[derive(Debug)]
pub enum SerializeError {
	Custom(String),
	ExceedsNumericLimits(std::num::TryFromIntError),
	Write(std::io::Error),
}

impl std::fmt::Display for SerializeError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SerializeError::Custom(message) => f.write_str(message),
			SerializeError::ExceedsNumericLimits(_) => f.write_str("value exceeds numeric limits"),
			SerializeError::Write(_) => f.write_str("could not write message"),
		}
	}
}

impl std::error::Error for SerializeError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			SerializeError::Custom(_) => None,
			SerializeError::ExceedsNumericLimits(err) => Some(err),
			SerializeError::Write(err) => Some(err),
		}
	}
}

impl serde::ser::Error for SerializeError {
	fn custom<T>(msg: T) -> Self where T: std::fmt::Display {
		SerializeError::Custom(msg.to_string())
	}
}
