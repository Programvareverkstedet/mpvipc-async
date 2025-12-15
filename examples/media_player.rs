use futures::StreamExt;
use mpvipc_async::{Event, Mpv, MpvDataType, MpvError, MpvExt, Property, parse_property};

fn seconds_to_hms(total: f64) -> String {
    let total = total as u64;
    let seconds = total % 60;
    let total = total / 60;
    let minutes = total % 60;
    let hours = total / 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}

#[tokio::main]
async fn main() -> Result<(), MpvError> {
    env_logger::init();

    let mpv = Mpv::connect("/tmp/mpv.sock").await?;

    mpv.observe_property(1, "path").await?;
    mpv.observe_property(2, "pause").await?;
    mpv.observe_property(3, "playback-time").await?;
    mpv.observe_property(4, "duration").await?;
    mpv.observe_property(5, "metadata").await?;

    let mut events = mpv.get_event_stream().await;
    while let Some(Ok(event)) = events.next().await {
        match event {
            mpvipc_async::Event::PropertyChange { name, data, .. } => {
                match parse_property(&name, data)? {
                    Property::Path(Some(value)) => println!("\nPlaying: {}", value),
                    Property::Pause(value) => {
                        println!("Pause: {}", value);
                    }
                    Property::PlaybackTime(Some(value)) => {
                        println!("Playback time: {}", seconds_to_hms(value));
                    }
                    Property::Duration(Some(value)) => {
                        println!("Duration: {}", seconds_to_hms(value));
                    }
                    Property::Metadata(Some(value)) => {
                        println!("File tags:");
                        if let Some(MpvDataType::String(value)) = value.get("ARTIST") {
                            println!(" Artist: {}", value);
                        }
                        if let Some(MpvDataType::String(value)) = value.get("ALBUM") {
                            println!(" Album: {}", value);
                        }
                        if let Some(MpvDataType::String(value)) = value.get("TITLE") {
                            println!(" Title: {}", value);
                        }
                        if let Some(MpvDataType::String(value)) = value.get("TRACK") {
                            println!(" Track: {}", value);
                        }
                    }
                    _ => (),
                }
            }
            Event::Shutdown => return Ok(()),
            Event::Unimplemented(_) => panic!("Unimplemented event"),
            _ => (),
        }
    }

    Ok(())
}
