# GVfs/GIO lazy initialization

Status: investigation complete; only a provably equivalent startup deferral
was implemented. The GVfs worker lifecycle is intentionally unchanged.

## Current startup path

1. `mounter::MOUNTERS` is a `LazyLock`. Its initializer builds a `Gvfs`
   mounter when the `gvfs` feature is enabled.
2. `Gvfs::new` spawns a dedicated OS thread, creates a GLib main loop, acquires
   `gio::VolumeMonitor::get()`, and connects mount/volume change callbacks.
3. `App::update_nav_model` used to call `MOUNTERS.is_empty()` while
   `App::init` was building the initial nav model. That forced the lazy
   registry, and therefore the GVfs thread and GIO monitor, to initialize
   during `App::init`.
4. `App::subscription` publishes every mounter's subscription. GVfs's
   subscription sends an initial `Cmd::Rescan` and forwards `Event::Changed`,
   `Event::Items`, mount results, authentication requests, and network results
   to the UI.

## Safe change made

`update_nav_model` now calls `mounter::has_mounter_backend()`. The helper is a
compile-time feature check, not a registry lookup, and is equivalent to
`!MOUNTERS.is_empty()` because `gvfs` is currently the only mounter backend.
This keeps the "Networks" nav entry behavior identical while preventing
`App::init` from creating the GVfs thread and GIO volume monitor.

`has_mounter_backend` is covered by a unit test. `src/lazy.rs` tests the
initialization state machine used for the compio runtime.

## Why the GVfs worker is not deferred further

The volume monitor subscription is not optional UI decoration:

- It supplies the initial mounted-drive/volume list used by the nav model and
  by desktop tabs that show mounted drives.
- Its GIO callbacks are the only source of mount/volume hotplug updates. If it
  is connected later, events in the gap are lost and the sidebar can remain
  stale.
- Network locations rely on the same worker for `network_scan`, `dir_info`,
  mounting, unmounting, and authentication. Starting on the first network
  open is too late for the sidebar's mount list and for desktop mounted drives.

`MOUNTERS` is first actually needed when the application publishes its
subscriptions, immediately after `Application::init`, before the first frame.
The first local folder view does not need GIO, but the same startup turn does
need the monitor for the nav model and hotplug subscription. Delaying that
until a later folder render would be a behavioral regression, not just a
resource optimization.

A future safe design would need to:

1. Separate `Gvfs::new` channel setup from worker startup (`OnceLock` or
   `Mutex<Option<Receiver>>` around the worker thread).
2. Start the worker when the subscription is first polled, and ensure every
   command path calls the same one-shot starter before sending.
3. Preserve the ordering guarantee that the volume monitor is connected before
   the initial `Cmd::Rescan` is processed.
4. Add integration coverage for initial mount enumeration plus mount-added,
   mount-removed, volume-added, and volume-removed events, and manually verify
   a network mount/unmount. A pure unit test cannot cover GLib main-context and
   D-Bus hotplug behavior.

Until that integration coverage exists, starting the worker any later is
speculative and risks silently breaking hotplug and the first nav/desktop view.

## Validation

- `cargo test lazy` covers one-time lazy initialization.
- `cargo test backend_presence_matches_compiled_features` covers the
  compile-time mounter-backend check.
- `cargo fmt --check` and `cargo check` results are recorded in the commit/task
  report. The full GVfs hotplug path is unchanged.
