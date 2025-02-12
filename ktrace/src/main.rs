use std::{
	io::Read,
	sync::{Arc, Condvar, Mutex, atomic::Ordering::Relaxed},
	time::Duration,
};

use app_state::AppState;
use clap::Parser;
use crossterm::event::{self, Event};
use ktrace_protocol::{Packet, TraceFilter};
use query_client::OobStream;

pub mod app_state;
pub mod query_client;
pub mod symbol_resolver;
pub mod view;
pub mod widget;

static INVALIDATED_MUTEX: Mutex<bool> = Mutex::new(false);
static INVALIDATED_CONDVAR: Condvar = Condvar::new();

pub fn invalidate() {
	*INVALIDATED_MUTEX.lock().unwrap() = true;
	INVALIDATED_CONDVAR.notify_all();
}

fn wait_for_invalidation() {
	let mut lock = INVALIDATED_MUTEX.lock().unwrap();
	while !*lock {
		lock = INVALIDATED_CONDVAR.wait(lock).unwrap();
	}
	*lock = false;
}

/// Starts the ktrace TUI frontend.
#[derive(Parser)]
struct Args {
	/// The socket path to connect to.
	#[clap(short = 's', long = "sock", default_value = ktrace_protocol::DEFAULT_SOCKET_PATH)]
	sock_path: String,
	/// The binaries to load. *Order matters*; symbols are resolved based on first-hit.
	/// If none are provided, only addresses are shown.
	binaries:  Vec<String>,
}

fn main() {
	let args = Args::parse();

	let mut terminal = ratatui::init();

	let resolver_client = symbol_resolver::run();

	for binary in args.binaries {
		resolver_client.add_binary(binary);
	}

	let app_state = Arc::new(AppState::new(resolver_client));

	std::thread::spawn(|| {
		loop {
			if let Ok(true) = event::poll(Duration::from_millis(100)) {
				invalidate();
			}
		}
	});

	std::thread::spawn({
		let app_state = app_state.clone();
		let sock_path = args.sock_path.clone();
		move || {
			struct StateOobStream(Arc<AppState>);

			impl OobStream for StateOobStream {
				fn on_connected(&self) {
					self.0.daemon_connected.set(true);
					invalidate();
				}

				fn on_disconnected(&self) {
					self.0.daemon_connected.set(false);
					invalidate();
				}
			}

			let client = Arc::new(query_client::run(
				sock_path,
				StateOobStream(app_state.clone()),
			));

			std::thread::spawn({
				let app_state = app_state.clone();
				let client = client.clone();

				move || {
					loop {
						let Ok(mut stream) = client.open_stream(0, None) else {
							std::thread::sleep(Duration::from_millis(100));
							continue;
						};

						{
							app_state.last_addresses.lock().unwrap().clear();
						}

						let mut buf = Box::new([0u8; 1024 * 1024 * 2]);
						let mut leftover = 0;

						while let Ok(nread) = stream.read(&mut buf[leftover..]) {
							if nread == 0 {
								break;
							}

							let total = nread + leftover;
							let addr_slice =
								unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u64, total / 8) };
							{
								let mut addrs = app_state.last_addresses.lock().unwrap();
								let push_base = addr_slice.len().saturating_sub(addrs.capacity());
								addrs.extend_from_slice(&addr_slice[push_base..]);
							}

							leftover = total & 7;
							if leftover != 0 {
								let leftover_base = total - leftover;
								buf.copy_within(leftover_base..total, 0);
							}

							invalidate();
						}
					}
				}
			});

			std::thread::spawn({
				let app_state = app_state.clone();
				let client = client.clone();

				move || {
					loop {
						let Ok(mut stream) = client.open_stream(0, Some(TraceFilter::LowerHalf)) else {
							std::thread::sleep(Duration::from_millis(100));
							continue;
						};

						{
							app_state.last_addresses.lock().unwrap().clear();
						}

						let mut buf = Box::new([0u8; 1024 * 1024 * 2]);
						let mut leftover = 0;

						while let Ok(nread) = stream.read(&mut buf[leftover..]) {
							if nread == 0 {
								break;
							}

							let total = nread + leftover;
							let addr_slice =
								unsafe { std::slice::from_raw_parts(buf.as_ptr() as *const u64, total / 8) };
							{
								let mut addrs = app_state.last_lower_addresses.lock().unwrap();
								let push_base = addr_slice.len().saturating_sub(addrs.capacity());
								addrs.extend_from_slice(&addr_slice[push_base..]);
							}

							leftover = total & 7;
							if leftover != 0 {
								let leftover_base = total - leftover;
								buf.copy_within(leftover_base..total, 0);
							}

							invalidate();
						}
					}
				}
			});

			loop {
				let mut should_invalidate = false;

				if let Some(Packet::InstCount { count }) =
					client.request(Packet::GetInstCount { thread_id: 0 })
				{
					app_state.instruction_count.store(count, Relaxed);
					should_invalidate = true;
				}

				if let Some(Packet::Status { status }) = client.request(Packet::GetStatus { thread_id: 0 }) {
					app_state.thread_status.store(status as usize, Relaxed);
					should_invalidate = true;
				}

				if should_invalidate {
					invalidate();
				}

				std::thread::sleep(Duration::from_millis(50));
			}
		}
	});

	loop {
		let ev = event::poll(Duration::from_millis(1))
			.unwrap_or_default()
			.then(|| event::read().unwrap());

		if matches!(ev, Some(Event::Key(_))) {
			break;
		}

		terminal
			.draw(|f| view::trace_log::draw(f, &app_state))
			.expect("failed to draw frame");

		wait_for_invalidation();
	}

	ratatui::restore();
}
