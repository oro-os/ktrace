use std::io;

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/ktrace-query.sock";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Packet {
	BadPacket,
	GetTraceLog { thread_id: u32, count: u64 },
	GetStatus { thread_id: u32 },
	GetInstCount { thread_id: u32 },
	Status { status: ThreadStatus },
	TraceLog { addresses: Vec<u64> },
	InstCount { count: usize },
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default)]
#[repr(usize)]
pub enum ThreadStatus {
	#[default]
	Idle    = 0,
	Running = 1,
	Dead    = 2,
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
