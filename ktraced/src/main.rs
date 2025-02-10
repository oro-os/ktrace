use std::{
	io::{self, BufWriter},
	os::unix::net::{UnixListener, UnixStream},
};

use clap::Parser;
use ktrace_plugin_protocol::{Packet, TracePackedWrite, TraceRead};
use log::{error, info, trace};

/// Runs the Kflame daemon, to which the QEMU plugin connects.
#[derive(Parser, Debug)]
struct Args {
	/// The path of the unix domain socket to listen on.
	#[clap(short = 's', long = "sock", default_value = ktrace_plugin_protocol::DEFAULT_SOCKET_PATH)]
	socket_path: String,
	/// The root directory for temporary trace files.
	#[clap(short = 'T', long = "tmpdir")]
	tmpdir:      Option<String>,
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
		std::thread::spawn({
			let tmpdir = args.tmpdir.clone();
			move || {
				if let Err(err) = handle_vcpu_stream(stream, tmpdir) {
					error!("error handling stream: {err:?}");
				}
			}
		});
	}
}

fn handle_vcpu_stream(stream: UnixStream, tmpdir: Option<String>) -> io::Result<()> {
	let packet_file = if let Some(tmpdir) = tmpdir {
		tempfile::tempfile_in(tmpdir)?
	} else {
		tempfile::tempfile()?
	};

	let mut out_file = BufWriter::new(packet_file.try_clone()?);

	let mut rd = std::io::BufReader::new(stream);

	loop {
		let msg = rd.read_packet()?;

		if let Packet::VcpuInit(vcpu) = &msg {
			info!("vcpu {} is online", vcpu.id);
		}

		out_file.write_packet_packed(&msg)?;
	}
}
