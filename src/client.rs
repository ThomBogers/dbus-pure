/// A D-Bus client.
pub struct Client {
	connection: crate::conn::Connection,
	last_serial: u32,
	name: Option<String>,
	received_messages: std::collections::VecDeque<(crate::types::MessageHeader<'static>, Option<crate::types::Variant<'static>>)>,
}

impl Client {
	/// Create a client that uses the given connection to a message bus.
	///
	/// This function will complete the `org.freedesktop.DBus.Hello` handshake and obtain its name before returning.
	pub fn new(connection: crate::conn::Connection) -> Result<Self, CreateClientError> {
		let mut client = Client {
			connection,
			last_serial: 0,
			name: None,
			received_messages: Default::default(),
		};

		client.name = Some(
			client.method_call(
				"org.freedesktop.DBus",
				crate::types::ObjectPath("/org/freedesktop/DBus".into()),
				"org.freedesktop.DBus",
				"Hello",
				None,
			)
			.map_err(CreateClientError::Hello)?
			.ok_or(None)
			.and_then(|body| body.into_string().map_err(Some))
			.map_err(CreateClientError::UnexpectedMessage)?
			.into_owned()
		);

		Ok(client)
	}

	/// Send a message with the given header and body.
	///
	/// - The header serial will be overwritten to a unique serial number, and does not need to be set to any specific value by the caller.
	///
	/// - Header fields corresponding to the required properties of the message type will be automatically inserted, and *must not* be inserted by the caller.
	///   For example, if `header.type` is `MethodCall { member, path }`, the `MessageHeaderField::Member` and `MessageHeaderField::Path` fields
	///   will be inserted automatically.
	///
	/// - The `MessageHeaderField::Sender` field will be automatically inserted and must not be inserted by the caller.
	///
	/// - The `MessageHeaderField::Signature` field will be automatically inserted if a body is specified, and must not be inserted by the caller.
	///
	/// Returns the serial of the message.
	pub fn send(&mut self, header: &mut crate::types::MessageHeader<'_>, body: Option<&crate::types::Variant<'_>>) -> Result<u32, crate::conn::SendError> {
		// Serial is in the range 1..=u32::max_value() , ie it rolls over to 1 rather than 0
		self.last_serial = self.last_serial % u32::max_value() + 1;
		header.serial = self.last_serial;

		if let Some(name) = &self.name {
			// name is cloned because the lifetime of self.name needs to be independent of the lifetime of header
			header.fields.to_mut().push(crate::types::MessageHeaderField::Sender(name.clone().into()));
		}

		let () = self.connection.send(header, body)?;

		Ok(self.last_serial)
	}

	/// A convenience wrapper around sending a `METHOD_CALL` message and receiving the corresponding `METHOD_RETURN` or `ERROR` response.
	///
	/// - If the method has zero parameters, set `parameters` to `None`.
	///
	/// - If the method has more than one parameter, set `parameters` to `Some(&Variant::Tuple { ... })`.
	///   For example, if the method takes two parameters of type string and byte, `parameters` should be
	///   `Some(&Variant::Tuple { elements: (&[Variant::String(...), Variant::U8(...)][..]).into() })`
	pub fn method_call(
		&mut self,
		destination: &str,
		path: crate::types::ObjectPath<'_>,
		interface: &str,
		member: &str,
		parameters: Option<&crate::types::Variant<'_>>,
	) -> Result<Option<crate::types::Variant<'static>>, MethodCallError> {
		let request_header_fields = &[
			crate::types::MessageHeaderField::Destination(destination.into()),
			crate::types::MessageHeaderField::Interface(interface.into()),
		][..];
		let mut request_header = crate::types::MessageHeader {
			r#type: crate::types::MessageType::MethodCall {
				member: member.into(),
				path,
			},
			flags: crate::types::message_flags::NONE,
			body_len: 0,
			serial: 0,
			fields: request_header_fields.into(),
		};

		self.send(&mut request_header, parameters).map_err(MethodCallError::SendRequest)?;

		let response = self.recv_matching(|header, _| {
			match header.r#type {
				crate::types::MessageType::Error { reply_serial, .. } if reply_serial == request_header.serial => true,
				crate::types::MessageType::MethodReturn { reply_serial, .. } if reply_serial == request_header.serial => true,
				_ => false,
			}
		}).map_err(MethodCallError::RecvResponse)?;

		match response.0.r#type {
			crate::types::MessageType::Error { name, reply_serial: _ } =>
				Err(MethodCallError::Error(name.into_owned(), response.1)),

			crate::types::MessageType::MethodReturn { reply_serial: _ } =>
				Ok(response.1),

			_ => unreachable!(),
		}
	}

	/// Receive a message from the message bus.
	///
	/// Blocks until a message is received.
	pub fn recv(&mut self) -> Result<(crate::types::MessageHeader<'static>, Option<crate::types::Variant<'static>>), crate::conn::RecvError> {
		if let Some(message) = self.received_messages.pop_front() {
			return Ok(message);
		}

		self.recv_new()
	}

	/// Receive a message from the message bus that satisfies the given predicate.
	///
	/// Messages that do not match the predicate will not be discarded. Instead they will be returned
	/// from subsequent calls to [`Client::recv`] or `recv_matching`.
	pub fn recv_matching(
		&mut self,
		mut predicate: impl FnMut(&crate::types::MessageHeader<'static>, Option<&crate::types::Variant<'static>>) -> bool,
	) -> Result<(crate::types::MessageHeader<'static>, Option<crate::types::Variant<'static>>), crate::conn::RecvError> {
		for (i, already_received_message) in self.received_messages.iter().enumerate() {
			if predicate(&already_received_message.0, already_received_message.1.as_ref()) {
				let result = self.received_messages.remove(i).unwrap();
				return Ok(result);
			}
		}

		loop {
			let (header, body) = self.recv_new()?;
			if predicate(&header, body.as_ref()) {
				return Ok((header, body));
			}

			self.received_messages.push_back((header, body));
		}
	}

	fn recv_new(&mut self) -> Result<(crate::types::MessageHeader<'static>, Option<crate::types::Variant<'static>>), crate::conn::RecvError> {
		self.connection.recv()
	}
}

impl std::fmt::Debug for Client {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Client")
			.field("connection", &())
			.field("last_serial", &self.last_serial)
			.field("name", &self.name)
			.finish()
	}
}

/// An error from creating a [`Client`].
#[derive(Debug)]
pub enum CreateClientError {
	Hello(MethodCallError),
	UnexpectedMessage(Option<crate::types::Variant<'static>>),
}

impl std::fmt::Display for CreateClientError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			CreateClientError::Hello(_) => f.write_str("could not complete hello"),
			CreateClientError::UnexpectedMessage(body) => write!(f, "hello request failed with {:?}", body),
		}
	}
}

impl std::error::Error for CreateClientError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			CreateClientError::Hello(err) => Some(err),
			CreateClientError::UnexpectedMessage(_) => None,
		}
	}
}

/// An error from calling a method using a [`Client`].
#[derive(Debug)]
pub enum MethodCallError {
	Error(String, Option<crate::types::Variant<'static>>),
	RecvResponse(crate::conn::RecvError),
	SendRequest(crate::conn::SendError),
}

impl std::fmt::Display for MethodCallError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			MethodCallError::Error(error_name, body) => write!(f, "method call failed with an error: {} {:?}", error_name, body),
			MethodCallError::RecvResponse(_) => f.write_str("could not receive response"),
			MethodCallError::SendRequest(_) => f.write_str("could not send request"),
		}
	}
}

impl std::error::Error for MethodCallError {
	fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
		match self {
			MethodCallError::Error(_, _) => None,
			MethodCallError::RecvResponse(err) => Some(err),
			MethodCallError::SendRequest(err) => Some(err),
		}
	}
}
