use std::{
	os::unix::net::UnixListener,
	sync::{Arc, OnceLock, mpsc::Sender},
};

use ktrace_protocol::{Packet, PacketDeserializer, PacketSerializer};

pub fn spawn(sock_path: String) -> QueryServer {
	let (master_send, master_recv) = std::sync::mpsc::channel();

	let this = QueryServer {
		master_send: master_send.clone(),
	};

	std::thread::spawn({
		move || {
			std::thread::spawn({
				let master_send = master_send.clone();
				move || {
					// Best-effort remove the socket file
					let _ = std::fs::remove_file(&sock_path);
					let sock = UnixListener::bind(&sock_path).expect("failed to bind to socket");

					for stream in sock.incoming() {
						let mut stream = stream.expect("failed to accept connection");
						let master_send = master_send.clone();

						std::thread::spawn(move || {
							loop {
								let req = stream
									.deserialize_packet()
									.expect("failed to deserialize packet");
								let res = Arc::new(OnceLock::new());

								master_send
									.send(MasterMessage::Client(ClientMessage {
										req,
										res: res.clone(),
									}))
									.expect("failed to send message to master");

								let res = res.wait();

								stream
									.serialize_packet(res)
									.expect("failed to serialize packet");
							}
						});
					}
				}
			});

			loop {
				match master_recv
					.recv()
					.expect("failed to receive master message")
				{
					MasterMessage::Connection(ConnectionMessage { res, .. }) => {
						res.set(master_send.clone())
							.expect("failed to set connection");
					}
					MasterMessage::Thread(ThreadMessage { message, .. }) => {
						match message {
							Message::Exit => {
								// TODO
							}
							Message::Idle => {
								// TODO
							}
							Message::Resume => {
								// TODO
							}
						}
					}
					MasterMessage::Client(ClientMessage { req, res }) => {
						match req {
							Packet::Ping(n) => {
								res.set(Packet::Pong(n)).unwrap();
							}
							Packet::Pong(_) => {}
						}
					}
				}
			}
		}
	});

	this
}

pub struct QueryServer {
	master_send: Sender<MasterMessage>,
}

impl QueryServer {
	pub fn new_thread(&self, thread_id: u32) -> QueryServerThread {
		let res = Arc::new(OnceLock::new());

		self.master_send
			.send(MasterMessage::Connection(ConnectionMessage {
				thread_id,
				res: res.clone(),
			}))
			.expect("failed to send message to master");

		QueryServerThread {
			thread_id,
			sender: res.wait().clone(),
		}
	}
}

enum MasterMessage {
	Connection(ConnectionMessage),
	Thread(ThreadMessage),
	Client(ClientMessage),
}

struct ConnectionMessage {
	thread_id: u32,
	res:       Arc<OnceLock<Sender<MasterMessage>>>,
}

struct ClientMessage {
	req: Packet,
	res: Arc<OnceLock<Packet>>,
}

pub struct QueryServerThread {
	thread_id: u32,
	sender:    Sender<MasterMessage>,
}

impl QueryServerThread {
	fn send(&self, msg: Message) {
		let _ = self.sender.send(MasterMessage::Thread(ThreadMessage {
			thread:  self.thread_id,
			message: msg,
		}));
	}

	pub fn idle(&self) {
		self.send(Message::Idle);
	}

	pub fn resume(&self) {
		self.send(Message::Resume);
	}

	pub fn exit(&self) {
		self.send(Message::Exit);
	}
}

impl Drop for QueryServerThread {
	fn drop(&mut self) {
		self.exit();
	}
}

struct ThreadMessage {
	thread:  u32,
	message: Message,
}

enum Message {
	Exit,
	Idle,
	Resume,
}
