use std::io::{Read, Write};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/ktrace.sock";

#[derive(Debug)]
#[repr(u8)]
pub enum Packet {
	VcpuInit(VcpuInit),
	VcpuResume(VcpuResume),
	VcpuIdle(VcpuIdle),
	VcpuExit(VcpuExit),
	Inst(Inst),
}

impl EnDec for Packet {
	fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
		let code = r.read_u8()?;
		match code {
			1 => Ok(Packet::VcpuInit(VcpuInit::read(r)?)),
			2 => Ok(Packet::VcpuResume(VcpuResume::read(r)?)),
			3 => Ok(Packet::VcpuIdle(VcpuIdle::read(r)?)),
			4 => Ok(Packet::VcpuExit(VcpuExit::read(r)?)),
			5 => Ok(Packet::Inst(Inst::read(r)?)),
			_ => {
				Err(std::io::Error::new(
					std::io::ErrorKind::InvalidData,
					"invalid packet code",
				))
			}
		}
	}

	fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
		match self {
			Packet::VcpuInit(v) => {
				w.write_u8(1)?;
				v.write(w)
			}
			Packet::VcpuResume(v) => {
				w.write_u8(2)?;
				v.write(w)
			}
			Packet::VcpuIdle(v) => {
				w.write_u8(3)?;
				v.write(w)
			}
			Packet::VcpuExit(v) => {
				w.write_u8(4)?;
				v.write(w)
			}
			Packet::Inst(v) => {
				w.write_u8(5)?;
				v.write(w)
			}
		}
	}
}

#[derive(Debug)]
#[repr(C)]
pub struct VcpuInit {
	pub id: u32,
}

impl EnDec for VcpuInit {
	fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
		Ok(VcpuInit {
			id: r.read_u32::<LittleEndian>()?,
		})
	}

	fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
		w.write_u32::<LittleEndian>(self.id)
	}
}

#[derive(Debug)]
#[repr(C)]
pub struct VcpuResume {
	pub id: u32,
}

impl EnDec for VcpuResume {
	fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
		Ok(VcpuResume {
			id: r.read_u32::<LittleEndian>()?,
		})
	}

	fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
		w.write_u32::<LittleEndian>(self.id)
	}
}

#[derive(Debug)]
#[repr(C)]
pub struct VcpuIdle {
	pub id: u32,
}

impl EnDec for VcpuIdle {
	fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
		Ok(VcpuIdle {
			id: r.read_u32::<LittleEndian>()?,
		})
	}

	fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
		w.write_u32::<LittleEndian>(self.id)
	}
}

#[derive(Debug)]
#[repr(C)]
pub struct VcpuExit {
	pub id: u32,
}

impl EnDec for VcpuExit {
	fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
		Ok(VcpuExit {
			id: r.read_u32::<LittleEndian>()?,
		})
	}

	fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
		w.write_u32::<LittleEndian>(self.id)
	}
}

#[derive(Debug)]
#[repr(C)]
pub struct Inst {
	pub addr: u64,
}

impl EnDec for Inst {
	fn read<R: Read>(r: &mut R) -> std::io::Result<Self> {
		Ok(Inst {
			addr: r.read_u64::<LittleEndian>()?,
		})
	}

	fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()> {
		w.write_u64::<LittleEndian>(self.addr)
	}
}

pub trait EnDec: Sized {
	fn write<W: Write>(&self, w: &mut W) -> std::io::Result<()>;
	fn read<R: Read>(r: &mut R) -> std::io::Result<Self>;
}

pub trait TraceRead: Read + Sized {
	#[inline]
	fn read_packet(&mut self) -> std::io::Result<Packet> {
		Packet::read(self)
	}
}

impl<T: Read + Sized> TraceRead for T {}

pub trait TraceWrite: Write + Sized {
	#[inline]
	fn write_packet(&mut self, packet: &Packet) -> std::io::Result<()> {
		packet.write(self)
	}
}

impl<T: Write + Sized> TraceWrite for T {}

pub trait TracePackedWrite: Write {
	fn write_packet_packed(&mut self, packet: &Packet) -> std::io::Result<()> {
		let p = unsafe {
			core::slice::from_raw_parts(
				packet as *const Packet as *const u8,
				core::mem::size_of::<Packet>(),
			)
		};
		self.write_all(p)?;
		Ok(())
	}
}

impl<T: Write + Sized> TracePackedWrite for T {}

pub trait TracePackedRead: Read {
	fn read_packet_packed(&mut self) -> std::io::Result<Packet> {
		let mut buf = [0u8; core::mem::size_of::<Packet>()];
		self.read_exact(&mut buf)?;
		Ok(unsafe { core::ptr::read(buf.as_ptr() as *const Packet) })
	}
}

impl<T: Read + Sized> TracePackedRead for T {}
