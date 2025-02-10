use std::os::unix::net::UnixListener;

use clap::Parser;
use ktrace_common::TraceRead;
use log::{info, trace};

/// Runs the Kflame daemon, to which the QEMU plugin connects.
#[derive(Parser, Debug)]
struct Args {
	/// The path of the unix domain socket to listen on.
	#[clap(short = 's', long, default_value = ktrace_common::DEFAULT_SOCKET_PATH)]
	socket_path: String,
}

fn main() {
	env_logger::builder()
		.filter_level(log::LevelFilter::Info)
		.init();

	let args = Args::parse();

	// Try to unlink it
	std::fs::remove_file(&args.socket_path).ok();

	let server_sock = UnixListener::bind(&args.socket_path).expect("failed to bind to socket");

	info!("listening on {}", args.socket_path);

	for stream in server_sock.incoming() {
		let stream = stream.expect("failed to accept connection");

		trace!("accepted connection");

		std::thread::spawn(move || {
			let mut rd = std::io::BufReader::new(stream);

			loop {
				let msg = rd.read_packet().expect("failed to read packet");
				info!("received packet: {:?}", msg);
			}
		});
	}
}
