use std::{
	io::{self, BufReader},
	os::unix::net::UnixStream,
	sync::{
		Arc, OnceLock,
		mpsc::{Receiver, Sender},
	},
	time::Duration,
};

use byteorder::{LittleEndian, ReadBytesExt};
use ktrace_protocol::{Packet, PacketDeserializer, PacketSerializer, TraceFilter};

#[derive(Debug)]
pub enum Message {
	Connected,
	Disconnected,
	Packet(Packet),
}

pub struct Client {
	socket_path: String,
	sender:      Sender<Request>,
}

impl Client {
	pub fn request(&self, req: Packet) -> Option<Packet> {
		debug_assert!(
			!matches!(req, Packet::OpenStream { .. }),
			"use open_stream instead"
		);

		let res = Arc::new(OnceLock::new());
		self.sender
			.send(Request {
				req,
				res: res.clone(),
			})
			.expect("failed to send request");
		res.wait().clone().take()
	}

	pub fn open_stream<const BUF_SZ: usize>(
		&self,
		thread_id: u32,
		filter: Option<TraceFilter>,
	) -> io::Result<Receiver<u64>> {
		let mut stream = UnixStream::connect(&self.socket_path)?;
		stream.serialize_packet(&Packet::OpenStream { thread_id, filter })?;
		let (sender, receiver) = std::sync::mpsc::sync_channel(BUF_SZ);

		std::thread::spawn(move || {
			let mut stream = BufReader::with_capacity(1024 * 1024 * 16, stream);

			while let Ok(addr) = stream.read_u64::<LittleEndian>() {
				if sender.send(addr).is_err() {
					break;
				}
			}
		});

		Ok(receiver)
	}
}

pub trait OobStream {
	fn on_connected(&self);
	fn on_disconnected(&self);
}

pub fn run<S: OobStream + Send + 'static>(sock_path: String, oob_stream: S) -> Client {
	let (sender, receiver) = std::sync::mpsc::channel();

	let this = Client {
		sender,
		socket_path: sock_path.clone(),
	};

	std::thread::spawn(move || {
		loop {
			let Ok(mut stream) = UnixStream::connect(&sock_path) else {
				std::thread::sleep(Duration::from_millis(100));
				continue;
			};

			oob_stream.on_connected();

			loop {
				let req: Request = receiver.recv().expect("failed to receive request");

				if let Err(_) = stream.serialize_packet(&req.req) {
					req.res.set(None).unwrap();
					break;
				}

				match stream.deserialize_packet() {
					Ok(r) => {
						req.res.set(Some(r)).unwrap();
					}
					Err(_) => {
						req.res.set(None).unwrap();
						break;
					}
				}
			}

			oob_stream.on_disconnected();
		}
	});

	this
}

struct Request {
	req: Packet,
	res: Arc<OnceLock<Option<Packet>>>,
}
