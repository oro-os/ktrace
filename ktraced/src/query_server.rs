use std::{
	collections::HashMap,
	fs::File,
	io::{Seek, SeekFrom},
	os::unix::net::UnixListener,
	sync::{
		Arc, OnceLock,
		atomic::{AtomicUsize, Ordering::Relaxed},
		mpsc::Sender,
	},
};

use byteorder::{LittleEndian, ReadBytesExt};
use ktrace_protocol::{Packet, PacketDeserializer, PacketSerializer, ThreadStatus};

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

			let mut threads = HashMap::new();

			loop {
				match master_recv
					.recv()
					.expect("failed to receive master message")
				{
					MasterMessage::Connection(ConnectionMessage { res, thread_state }) => {
						threads.insert(thread_state.id, thread_state);

						res.set(master_send.clone())
							.expect("failed to set connection");
					}
					MasterMessage::Thread(ThreadMessage { message, thread }) => {
						match message {
							Message::Exit => {
								let _ = threads.remove(&thread);
							}
							Message::Idle => {
								threads.get_mut(&thread).map(|state| {
									state.status = ThreadStatus::Idle;
								});
							}
							Message::Resume => {
								threads.get_mut(&thread).map(|state| {
									state.status = ThreadStatus::Running;
								});
							}
						}
					}
					MasterMessage::Client(ClientMessage { req, res }) => {
						match req {
							Packet::GetTraceLog { count, thread_id } => {
								let mut addresses = Vec::with_capacity(count as usize);

								if let Some(state) = threads.get_mut(&thread_id) {
									let size =
										state.temp_file.metadata().map(|m| m.len()).unwrap_or(0)
											/ 8;
									let base = size.saturating_sub(count);
									if let Ok(_) = state.temp_file.seek(SeekFrom::Start(base * 8)) {
										for _ in 0..count {
											if let Ok(addr) =
												state.temp_file.read_u64::<LittleEndian>()
											{
												addresses.push(addr);
											} else {
												break;
											}
										}
									}
								}

								res.set(Packet::TraceLog { addresses })
									.expect("failed to set response");
							}
							Packet::GetStatus { thread_id } => {
								let status = threads
									.get(&thread_id)
									.map(|state| state.status)
									.unwrap_or(ThreadStatus::Dead);

								res.set(Packet::Status { status })
									.expect("failed to set response");
							}
							Packet::GetInstCount { thread_id } => {
								let count = threads
									.get(&thread_id)
									.map(|state| state.addr_counter.load(Relaxed))
									.unwrap_or(0);

								res.set(Packet::InstCount { count })
									.expect("failed to set response");
							}
							_ => {
								res.set(Packet::BadPacket).expect("failed to set response");
							}
						}
					}
				}
			}
		}
	});

	this
}

pub struct ThreadState {
	pub id:           u32,
	pub temp_file:    File,
	pub addr_counter: Arc<AtomicUsize>,
	pub status:       ThreadStatus,
}

pub struct QueryServer {
	master_send: Sender<MasterMessage>,
}

impl QueryServer {
	pub fn new_thread(&self, thread_state: ThreadState) -> QueryServerThread {
		let res = Arc::new(OnceLock::new());
		let thread_id = thread_state.id;

		self.master_send
			.send(MasterMessage::Connection(ConnectionMessage {
				thread_state,
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
	thread_state: ThreadState,
	res:          Arc<OnceLock<Sender<MasterMessage>>>,
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
