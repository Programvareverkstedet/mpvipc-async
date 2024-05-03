[![Coverage](https://pvv.ntnu.no/~oysteikt/gitea/mpvipc/master/coverage/badges/for_the_badge.svg)](https://pvv.ntnu.no/~oysteikt/gitea/mpvipc/master/coverage/src/)
[![Docs](https://img.shields.io/badge/docs-blue?style=for-the-badge&logo=rust)](https://pvv.ntnu.no/~oysteikt/gitea/mpvipc/master/docs/mpvipc/)

# mpvipc

A small library which provides bindings to control existing mpv instances through sockets.

## Dependencies

- `mpv`
- `cargo` (make dependency)
- `cargo-nextest` (test depencency)
- `grcov` (test depencency)

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
    mpv.set_property("pause", !paused).expect("Error pausing");
}
```