use std::io;

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Packet {
	Ping(u64),
	Pong(u64),
}

pub trait PacketSerializer: io::Write + Sized {
	fn serialize_packet(&mut self, packet: &Packet) -> io::Result<()> {
		packet
			.serialize(&mut Serializer::new(self))
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
		Ok(())
	}
}

impl<T: io::Write> PacketSerializer for T {}

pub trait PacketDeserializer: io::Read + Sized {
	fn deserialize_packet(&mut self) -> io::Result<Packet> {
		Packet::deserialize(&mut Deserializer::new(self))
			.map_err(|e| io::Error::new(io::ErrorKind::Other, e))
	}
}
