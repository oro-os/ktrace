# Ktrace

<img align="right" src="screenshot.gif" width="50%" />

**Ktrace** is a low level (typically kernel) instruction tracing tool suite.

> ![NOTE]
> Ktrace is at a very, very early stage of development and is typically only developed on an as-needed
> basis when issues arise developing the [Oro kernel](https://github.com/oro-os/kernel). If something is
> missing, and you'd like to have it added, do not hesitate to submit a pull request!

- `ktrace` is a TUI program for displaying traces and configuring filters.
- `ktraced` is a trace collection agent that liaises between `ktrace` and `libktrace_plugin.so`.
- `libktrace_plugin.so` is a QEMU plugin that streams instruction accesses to `ktraced`.

In theory, other instruction tracing tools can stream to `ktraced`, and other frontends can consume
the trace data too:

- `ktrace-plugin-protocol` is a binary protocol for streaming trace data interleaved with core data
  to the `ktraced` daemon.
- `ktrace-protocol` is a msgpack-based protocol (also binary) for interacting with `ktraced` as a
  frontend.

Note that `ktraced` does not do symbol resolution; its only task is to do low-level address- and thread-based
filtering and querying of the address data. Frontends must perform symbol resolution and display on their own,
including higher-level filtering. Typically, the frontend will lower a high-level filter to an address-/thread-based
'pre-filter' for `ktraced`, and then use a higher level 'post-filter' on the frontend to further filter those results.

# License
Copyright &copy; 2025, Joshua Lee Junon.

Released under the MIT License OR the Apache 2.0 License, at the user's discretion.

Part of the [Oro operating system](https://github.com/oro-os) project.
