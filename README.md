[![Coverage](https://pages.pvv.ntnu.no/Grzegorz/mpvipc-async/main/coverage/badges/for_the_badge.svg)](https://pages.pvv.ntnu.no/Grzegorz/mpvipc-async/main/coverage/src/)
[![Docs](https://img.shields.io/badge/docs-blue?style=for-the-badge&logo=rust)](https://pages.pvv.ntnu.no/Grzegorz/mpvipc-async/main/docs/mpvipc_async/)

# mpvipc-async

> [!NOTE]
> This is a fork of [gitlab.com/mpv-ipc/mpvipc](https://gitlab.com/mpv-ipc/mpvipc).
> The fork adds support for use in asynchronous contexts.

A small library which provides bindings to control existing mpv instances through sockets.

## Dependencies

- `mpv` (runtime dependency)
- `cargo-nextest` (optional test depencency)
- `grcov` (optional test depencency)

## Example

Make sure mpv is started with the following option:

```bash
$ mpv --input-ipc-server=/tmp/mpv.sock --idle
```

Here is a small code example which connects to the socket `/tmp/mpv.sock` and toggles playback.

```rust
use mpvipc_async::*;

#[tokio::main]
async fn main() -> Result<(), MpvError> {
    let mpv = Mpv::connect("/tmp/mpv.sock").await?;
    let paused: bool = mpv.get_property("pause").await?;
    mpv.set_property("pause", !paused).await.expect("Error pausing");
}
```
