use std::{
	io::{self, BufWriter, Write},
	os::unix::net::{UnixListener, UnixStream},
	sync::{
		Arc,
		atomic::{AtomicUsize, Ordering::Relaxed},
	},
};

mod query_server;

use byteorder::{LittleEndian, WriteBytesExt};
use clap::Parser;
use ktrace_plugin_protocol::{Packet, TraceRead};
use log::{error, info, trace};
use query_server::ThreadState;

/// Runs the Kflame daemon, to which the QEMU plugin connects.
#[derive(Parser, Debug)]
struct Args {
	/// The path of the unix domain socket to listen on for trace connections (e.g. from QEMU or other plugins)
	#[clap(short = 's', long = "trace-sock", default_value = ktrace_plugin_protocol::DEFAULT_SOCKET_PATH)]
	socket_path: String,
	/// The path of the unix domain socket to listen on for query connections (e.g. the ktrace client)
	#[clap(short = 'b', long = "sock", default_value = ktrace_protocol::DEFAULT_SOCKET_PATH)]
	query_socket_path: String,
	/// The root directory for temporary trace files.
	#[clap(short = 'T', long = "tmpdir")]
	tmpdir: Option<String>,
	/// Show verbose logs.
	#[clap(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
	verbose: usize,
}

fn main() {
	let args = Args::parse();

	env_logger::builder()
		.filter_level(match args.verbose {
			0 => log::LevelFilter::Info,
			1 => log::LevelFilter::Debug,
			_ => log::LevelFilter::Trace,
		})
		.init();

	// Try to unlink it
	std::fs::remove_file(&args.socket_path).ok();

	let server_sock = UnixListener::bind(&args.socket_path).expect("failed to bind to socket");

	info!("listening for trace connections at '{}'", args.socket_path);

	let query_serv = Arc::new(query_server::spawn(args.query_socket_path.clone()));

	info!(
		"listening for query connections at '{}'",
		args.query_socket_path
	);

	for stream in server_sock.incoming() {
		let stream = stream.expect("failed to accept connection");

		trace!("accepted connection");
		std::thread::spawn({
			let tmpdir = args.tmpdir.clone();
			let query_serv = query_serv.clone();
			move || {
				if let Err(err) = handle_vcpu_stream(stream, tmpdir, query_serv) {
					error!("error handling stream: {err:?}");
				}
			}
		});
	}
}

fn handle_vcpu_stream(
	stream: UnixStream,
	tmpdir: Option<String>,
	query_serv: Arc<query_server::QueryServer>,
) -> io::Result<()> {
	let mut tf = tempfile::Builder::new();
	tf.append(true);

	let addr_file = if let Some(tmpdir) = tmpdir {
		tf.tempfile_in(tmpdir)?
	} else {
		tf.tempfile()?
	};

	let mut out_file = BufWriter::new(addr_file.reopen()?);

	let mut rd = std::io::BufReader::new(stream);

	let msg = rd.read_packet()?;
	let Packet::VcpuInit(vcpu) = msg else {
		panic!("expected VcpuInit, got {msg:?}");
	};

	let addr_counter = Arc::new(AtomicUsize::new(0));

	let client = query_serv.new_thread(ThreadState {
		id:           vcpu.id,
		addr_counter: addr_counter.clone(),
		temp_file:    addr_file.reopen()?,
		status:       Default::default(),
	});

	info!("received VcpuInit for vcpu {}", vcpu.id);

	loop {
		match rd.read_packet()? {
			Packet::VcpuResume => {
				out_file.flush()?;
				client.resume();
			}
			Packet::VcpuIdle => {
				out_file.flush()?;
				client.idle();
			}
			Packet::VcpuExit => {
				out_file.flush()?;
				client.exit();
				break;
			}
			Packet::Inst(inst) => {
				out_file.write_u64::<LittleEndian>(inst.addr)?;
				addr_counter.fetch_add(1, Relaxed);
			}
			msg => {
				panic!("unexpected message: {:?}", msg);
			}
		}
	}

	Ok(())
}
