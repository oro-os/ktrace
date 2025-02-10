use std::{
	sync::{Arc, Condvar, Mutex},
	time::Duration,
};

use app_state::AppState;
use clap::Parser;
use crossterm::event::{self, Event};
use ktrace_protocol::Packet;
use query_client::OobStream;

pub mod app_state;
pub mod query_client;
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
}

fn main() {
	let args = Args::parse();

	let app_state = Arc::new(AppState::default());
	let mut terminal = ratatui::init();

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
					self.0.set_daemon_connected(true);
					invalidate();
				}

				fn on_disconnected(&self) {
					self.0.set_daemon_connected(false);
					invalidate();
				}
			}

			let client = query_client::run(sock_path, StateOobStream(app_state.clone()));

			loop {
				std::thread::sleep(Duration::from_secs(1));
				let _ = client.request(Packet::Ping(1337));
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
			.draw(|f| view::primary::draw(f, &app_state))
			.expect("failed to draw frame");

		wait_for_invalidation();
	}

	ratatui::restore();
}
