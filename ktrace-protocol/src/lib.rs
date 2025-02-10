use std::io;

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/ktrace-query.sock";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Packet {
	BadPacket,
	GetTraceLog { count: u64, thread_id: u32 },
	TraceLog { addresses: Vec<u64> },
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

impl<T: io::Read> PacketDeserializer for T {}
