use std::{fmt, io};

use rmp_serde::{Deserializer, Serializer};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/ktrace-query.sock";

#[derive(Serialize, Deserialize, Clone, Copy, thiserror::Error, Debug)]
#[repr(u8)]
pub enum Error {
	#[error("bad packet")]
	BadPacket = 1,
	#[error("invalid thread id")]
	BadThread = 2,
}

#[derive(Serialize, Deserialize, Clone)]
#[repr(u8)]
pub enum Packet {
	Error(Error),
	GetTraceLog {
		thread_id: u32,
		count:     u64,
		filter:    Option<TraceFilter>,
	},
	GetStatus {
		thread_id: u32,
	},
	GetInstCount {
		thread_id: u32,
	},
	Status {
		status: ThreadStatus,
	},
	TraceLog {
		addresses: Vec<u64>,
	},
	InstCount {
		count: usize,
	},
	OpenStream {
		thread_id: u32,
		filter:    Option<TraceFilter>,
	},
}

impl fmt::Debug for Packet {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Packet::Error(err) => write!(f, "Error {{ err: {err:?} }}"),
			Packet::GetTraceLog {
				thread_id,
				count,
				filter,
			} => {
				write!(
					f,
					"GetTraceLog {{ thread_id: {thread_id:?}, count: {count:?}, filter: {filter:?} }}",
				)
			}
			Packet::GetStatus { thread_id } => {
				write!(f, "GetStatus {{ thread_id: {thread_id:?} }}")
			}
			Packet::GetInstCount { thread_id } => {
				write!(f, "GetInstCount {{ thread_id: {thread_id:?} }}")
			}
			Packet::Status { status } => write!(f, "Status {{ status: {status:?} }}"),
			Packet::TraceLog { addresses } => {
				write!(f, "TraceLog {{ addresses: {addresses:?} }}")
			}
			Packet::InstCount { count } => write!(f, "InstCount {{ count: {count:?} }}"),
			Packet::OpenStream { filter, thread_id } => {
				write!(
					f,
					"OpenStream {{ thread_id: {thread_id:?}, filter: {filter:?} }}"
				)
			}
		}
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum TraceFilter {
	LowerHalf,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, Default, PartialEq)]
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
		Packet::deserialize(&mut Deserializer::new(self)).map_err(|e| io::Error::new(io::ErrorKind::Other, e))
	}
}

impl<T: io::Read> PacketDeserializer for T {}
