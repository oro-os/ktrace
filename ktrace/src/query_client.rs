use std::{
	os::unix::net::UnixStream,
	sync::{Arc, OnceLock, mpsc::Sender},
	time::Duration,
};

use ktrace_protocol::{Packet, PacketDeserializer, PacketSerializer};

#[derive(Debug)]
pub enum Message {
	Connected,
	Disconnected,
	Packet(Packet),
}

pub struct Client {
	sender: Sender<Request>,
}

impl Client {
	pub fn request(&self, req: Packet) -> Option<Packet> {
		let res = Arc::new(OnceLock::new());
		self.sender
			.send(Request {
				req,
				res: res.clone(),
			})
			.expect("failed to send request");
		res.wait().clone().take()
	}
}

pub trait OobStream {
	fn on_connected(&self);
	fn on_disconnected(&self);
}

pub fn run<S: OobStream + Send + 'static>(sock_path: String, oob_stream: S) -> Client {
	let (sender, receiver) = std::sync::mpsc::channel();

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

	Client { sender }
}

struct Request {
	req: Packet,
	res: Arc<OnceLock<Option<Packet>>>,
}
