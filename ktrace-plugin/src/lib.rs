#![feature(sync_unsafe_cell, ptr_as_ref_unchecked)]

use std::{
	cell::SyncUnsafeCell,
	collections::HashMap,
	io::{BufWriter, Write},
	os::unix::net::UnixStream,
	sync::{Arc, Mutex},
};

use anyhow::Result;
use ctor::ctor;
use ktrace_plugin_protocol::{Inst, Packet, TraceWrite, VcpuInit};
use qemu_plugin::{
	CallbackFlags, PluginId, TranslationBlock, VCPUIndex,
	install::{Args, Info, Value},
	plugin::{HasCallbacks, PLUGIN, Plugin, Register},
};

struct Vcpu {
	trace: SyncUnsafeCell<BufWriter<UnixStream>>,
}

#[derive(Default)]
struct Ktrace {
	socket_path: String,
	vcpus:       Arc<HashMap<VCPUIndex, Vcpu>>,
}

impl Register for Ktrace {
	fn register(&mut self, _id: PluginId, args: &Args, _info: &Info) -> Result<()> {
		self.socket_path = if let Some(Value::String(v)) = args.parsed.get("sock") {
			v.clone()
		} else {
			ktrace_plugin_protocol::DEFAULT_SOCKET_PATH.to_string()
		};

		println!("ktrace: socket path is {}", self.socket_path);

		Ok(())
	}
}

impl HasCallbacks for Ktrace {
	fn on_vcpu_init(&mut self, _id: PluginId, vcpu_id: VCPUIndex) -> Result<()> {
		let vcpu = match self.vcpus.get(&vcpu_id) {
			Some(v) => v,
			None => {
				Arc::get_mut(&mut self.vcpus)
					.expect("failed to get mutable reference to vcpus")
					.insert(
						vcpu_id,
						Vcpu {
							trace: SyncUnsafeCell::new(BufWriter::new(UnixStream::connect(
								&self.socket_path,
							)?)),
						},
					);

				self.vcpus.get(&vcpu_id).unwrap()
			}
		};

		let sock = unsafe { vcpu.trace.get().as_mut_unchecked() };
		sock.write_packet(&Packet::VcpuInit(VcpuInit { id: vcpu_id.into() }))?;
		sock.flush()?;

		Ok(())
	}

	fn on_vcpu_resume(&mut self, _id: PluginId, vcpu_id: VCPUIndex) -> Result<()> {
		let vcpu = self.vcpus.get(&vcpu_id).expect("vcpu not found");
		let sock = unsafe { vcpu.trace.get().as_mut_unchecked() };
		sock.write_packet(&Packet::VcpuResume)?;
		sock.flush()?;
		Ok(())
	}

	fn on_vcpu_idle(&mut self, _id: PluginId, vcpu_id: VCPUIndex) -> Result<()> {
		let vcpu = self.vcpus.get(&vcpu_id).expect("vcpu not found");
		let sock = unsafe { vcpu.trace.get().as_mut_unchecked() };
		sock.write_packet(&Packet::VcpuIdle)?;
		sock.flush()?;
		Ok(())
	}

	fn on_vcpu_exit(&mut self, _id: PluginId, vcpu_id: VCPUIndex) -> std::result::Result<(), anyhow::Error> {
		let vcpu = self.vcpus.get(&vcpu_id).expect("vcpu not found");
		unsafe { vcpu.trace.get().as_mut_unchecked() }.write_packet(&Packet::VcpuExit)?;
		Ok(())
	}

	fn on_translation_block_translate(&mut self, _id: PluginId, tb: TranslationBlock) -> Result<()> {
		for insn in tb.instructions() {
			let vcpus = self.vcpus.clone();
			let addr = insn.vaddr();

			insn.register_execute_callback_flags(
				move |vcpu_idx| {
					let vcpu = vcpus
						.get(&vcpu_idx)
						.expect("instruction executed on unregistered vcpu");

					unsafe { vcpu.trace.get().as_mut_unchecked() }
						.write_packet(&Packet::Inst(Inst { addr }))
						.expect("failed to write instruction");
				},
				CallbackFlags::QEMU_PLUGIN_CB_NO_REGS,
			);
		}

		Ok(())
	}
}

impl Plugin for Ktrace {}

#[ctor]
fn init() {
	PLUGIN
		.set(Mutex::new(Box::new(Ktrace::default())))
		.map_err(|_| anyhow::anyhow!("failed to set plugin Ktrace"))
		.expect("failed to set plugin Ktrace");
}
