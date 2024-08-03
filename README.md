[![Coverage](https://pvv.ntnu.no/~oysteikt/gitea/mpvipc/master/coverage/badges/for_the_badge.svg)](https://pvv.ntnu.no/~oysteikt/gitea/mpvipc/master/coverage/src/)
[![Docs](https://img.shields.io/badge/docs-blue?style=for-the-badge&logo=rust)](https://pvv.ntnu.no/~oysteikt/gitea/mpvipc/master/docs/mpvipc/)

# mpvipc

> **NOTE:** This is a fork of [gitlab.com/mpv-ipc/mpvipc](https://gitlab.com/mpv-ipc/mpvipc), which introduces a lot of changes to be able to use the library asynchronously with [tokio](https://github.com/tokio-rs/tokio).

---

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
use mpvipc::*;

#[tokio::main]
async fn main() -> Result<(), MpvError> {
    let mpv = Mpv::connect("/tmp/mpv.sock").await?;
    let paused: bool = mpv.get_property("pause").await?;
    mpv.set_property("pause", !paused).await.expect("Error pausing");
}
```