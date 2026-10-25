# Local ASIO host dependency

Upstream: maolan-asio 0.0.1, https://github.com/maolan/asio and the published crates.io package.
Package SHA-256: `B1C08352937A4A2392BE4E664F583EC8841A469D24408BFAECBF98442613EA11`.
License: BSD-2-Clause; retain LICENSE. No SDK or device-driver binaries are included.

OpenTrack reviewed the COM creation/ownership, registry discovery, ABI layouts, sample conversion, callback storage, buffer negotiation, and stream shutdown paths. This is an early dependency; tests and the local Mackie capability query do not establish general driver compatibility.

Local changes:

- Correct buffer granularity: -1 doubles from minimum, 0 uses the preferred fixed size, positive values step from minimum. Reference: ASIO specification section ASIOGetBufferSize, and the vendor clarification https://forums.steinberg.net/t/clarification-on-asio-spec-for-buffer-size-granulatity/201896.
- Isolate mutable audio scratch/closure behind a nonblocking callback gate and UnsafeCell. Notification events and sample rate use atomics, so notifications do not create concurrent mutable references to the stream state.
- Keep the host callback function table in the pinned stream state for the complete stream lifetime, rather than passing a stack-local table.
- Validate callback indices/null time pointers and report the actual fallback buffer size.
- Enumerate only the native process registry view; x64 OpenTrack cannot load an x86 in-process driver.
- Expose driver latency queries after stream buffer allocation.

The adapter uses one dedicated COM owner thread, bounded preallocated stereo buffers, shared capture for metering/monitoring/recording, and stops/disposes on that same thread. Driver reset/rate/buffer/latency events stop the session and leave the project available for reconnect. Physical monitoring, sustained stability, control-panel behavior, hotplug, coexistence, and actual loopback latency still require hardware acceptance.
