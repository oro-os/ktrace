use std::{
	collections::HashMap,
	fs::File,
	io::{Cursor, Read, Seek, SeekFrom},
	os::unix::net::{UnixListener, UnixStream},
	sync::{
		Arc, OnceLock,
		atomic::{AtomicUsize, Ordering::Relaxed},
		mpsc::Sender,
	},
};

use byteorder::{LittleEndian, ReadBytesExt};
use ktrace_protocol::{
	Error as PacketError, Packet, PacketDeserializer, PacketSerializer, ThreadStatus, TraceFilter,
};
use log::trace;

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

								if let Packet::OpenStream { thread_id, filter } = &req {
									master_send
										.send(MasterMessage::OpenStream(OpenStreamMessage {
											stream,
											thread_id: *thread_id,
											filter: *filter,
										}))
										.expect("failed to send message to master");
									return;
								}

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
				let req = master_recv
					.recv()
					.expect("failed to receive master message");

				match req {
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
					MasterMessage::OpenStream(OpenStreamMessage {
						mut stream,
						thread_id,
						filter,
					}) => {
						let Some(mut file) = threads
							.get(&thread_id)
							.map(|state| state.temp_file.try_clone().expect("failed to clone file"))
						else {
							// Best effort.
							let _ = stream.serialize_packet(&Packet::Error(PacketError::BadThread));
							continue;
						};

						std::thread::spawn(move || {
							let mut counter = 0;

							file.seek(SeekFrom::Start(0))
								.expect("failed to seek to start");

							let mut buffer = [0u8; 4096 * 16];

							loop {
								let mut results = Vec::with_capacity(buffer.len() / 8);

								let size = file.metadata().map(|m| m.len()).unwrap_or(0) / 8;
								// At least 1 so that we block until one is available.
								let available = (size - counter).min((buffer.len() as u64) / 8).max(1);
								counter += available;

								file.read_exact(&mut buffer[..(available as usize * 8)])
									.expect("failed to read from file");

								let mut cursor = Cursor::new(&buffer[..(available as usize * 8)]);

								for _ in 0..available {
									if let Ok(addr) = cursor.read_u64::<LittleEndian>() {
										let include = match filter {
											None => true,
											Some(TraceFilter::LowerHalf) => addr & 0x8000_0000_0000_0000 == 0,
										};

										if include {
											results.push(addr);
										}
									} else {
										break;
									}
								}

								if !results.is_empty() {
									stream
										.serialize_packet(&Packet::TraceLog { addresses: results })
										.expect("failed to send trace log");
								}
							}
						});
					}
					MasterMessage::Client(ClientMessage { req, res }) => {
						trace!("<-- {req:?}");

						macro_rules! respond {
							($res:expr, $packet:expr) => {
								let packet = $packet;
								trace!("--> {packet:?}");
								$res.set(packet).expect("failed to set response");
							};
						}

						match req {
							Packet::GetTraceLog {
								count,
								thread_id,
								filter,
							} => {
								let mut addresses = Vec::with_capacity(count as usize);

								if let Some(state) = threads.get_mut(&thread_id) {
									let size = state.temp_file.metadata().map(|m| m.len()).unwrap_or(0) / 8;

									match filter {
										None => {
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
										Some(TraceFilter::LowerHalf) => {
											// NOTE(qix-): This is a little naive; not all implementations
											// NOTE(qix-): will have the high bit set for higher-half addresses.
											const MASK: u64 = 0x8000_0000_0000_0000;

											// We traverse backward, potentially quite slowly, to find
											// the latest N addresses that match the filter. Then we reverse
											// the vector.
											let mut cur_base =
												size.min(state.last_lower_half.load(Relaxed) as u64 + 1);

											let mut buf = [0u8; 4096];
											let mut found_buf = [0u64; 4096 / 8];
											while addresses.len() < count as usize {
												if cur_base == 0 {
													break;
												}

												let new_base =
													cur_base.saturating_sub((buf.len() / 8) as u64);
												let search_count = cur_base - new_base;
												cur_base = new_base;

												if let Ok(_) =
													state.temp_file.seek(SeekFrom::Start(cur_base * 8))
												{
													if let Ok(_) = state
														.temp_file
														.read_exact(&mut buf[..((search_count * 8) as usize)])
													{
														let mut cursor = Cursor::new(
															&buf[..((search_count * 8) as usize)],
														);
														let mut found = 0;

														for _ in 0..search_count {
															if let Ok(addr) =
																cursor.read_u64::<LittleEndian>()
															{
																if addr & MASK == 0 && addr != 0 {
																	found_buf[found] = addr;
																	found += 1;
																}
															} else {
																break;
															}
														}

														if found > 0 {
															found_buf[..found].reverse();
															addresses.extend_from_slice(&found_buf[..found]);
														}
													} else {
														break;
													}
												} else {
													break;
												}
											}
										}
									}
								}

								respond!(res, Packet::TraceLog { addresses });
							}
							Packet::GetStatus { thread_id } => {
								let status = threads
									.get(&thread_id)
									.map(|state| state.status)
									.unwrap_or(ThreadStatus::Dead);

								respond!(res, Packet::Status { status });
							}
							Packet::GetInstCount { thread_id } => {
								let count = threads
									.get(&thread_id)
									.map(|state| state.addr_counter.load(Relaxed))
									.unwrap_or(0);

								respond!(res, Packet::InstCount { count });
							}
							Packet::OpenStream { .. } => {
								unreachable!()
							}
							_ => {
								res.set(Packet::Error(PacketError::BadPacket))
									.expect("failed to set response");
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
	pub id: u32,
	pub temp_file: File,
	pub addr_counter: Arc<AtomicUsize>,
	pub last_lower_half: Arc<AtomicUsize>,
	pub status: ThreadStatus,
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
	OpenStream(OpenStreamMessage),
}

struct OpenStreamMessage {
	thread_id: u32,
	stream:    UnixStream,
	filter:    Option<TraceFilter>,
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
