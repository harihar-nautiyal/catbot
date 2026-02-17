use anyhow::Result;
use catbot::invite::on_room_invite;
use catbot::models::joke::Joke;
use dotenv::dotenv;
use matrix_sdk::room::Room;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::RoomMessageEventContent;
use matrix_sdk::{Client, config::SyncSettings};
use ruma::UserId;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let server = env::var("SERVER").expect("SERVER environment variable is not set.");
    let username = env::var("USERNAME").expect("USERNAME environment variable is not set.");
    let password = env::var("PASSWORD").expect("PASSWORD environment variable is not set.");

    let user_id_str = format!("@{}:{}", username, server);
    let user = UserId::parse(&user_id_str)?;

    let client = Client::builder()
        .server_name(user.server_name())
        .build()
        .await?;

    client
        .matrix_auth()
        .login_username(&username, &password)
        .send()
        .await?;

    println!("Logged in successfully!");

    for room in client.invited_rooms() {
        println!("Joining invited room: {}", room.room_id());
        let _ = room.join().await;
    }

    client.add_event_handler(
        |ev: OriginalSyncRoomMessageEvent, room: matrix_sdk::room::Room| async move {
            let sender = ev.sender.to_string();
            let body = ev.content.body();

            println!("{} -> {}", sender, body);

            if body.starts_with("!cat") {
                let content = RoomMessageEventContent::text_plain("meow");

                if let Err(e) = room.send(content).await {
                    eprintln!("failed to send message: {:?}", e);
                }
            }

            if body.starts_with("!joke") {
                let joke_text = async {
                    let resp =
                        reqwest::get("https://v2.jokeapi.dev/joke/Programming?type=single").await;
                    match resp {
                        Ok(resp) => {
                            let parsed = resp.json::<Joke>().await;
                            match parsed {
                                Ok(j) => {
                                    if let Some(j_line) = j.joke {
                                        j_line
                                    } else if let (Some(setup), Some(delivery)) =
                                        (j.setup, j.delivery)
                                    {
                                        format!("{} … {}", setup, delivery)
                                    } else {
                                        "No joke found :(".to_string()
                                    }
                                }
                                Err(_) => "Failed to parse joke :(".to_string(),
                            }
                        }
                        Err(_) => "Failed to fetch joke :(".to_string(),
                    }
                }
                .await;

                let content = RoomMessageEventContent::text_plain(joke_text);

                if let Err(e) = room.send(content).await {
                    eprintln!("failed to send joke: {:?}", e);
                }
            }
        },
    );

    client.add_event_handler(on_room_invite);

    let sync_settings = SyncSettings::default()
        .ignore_timeout_on_first_sync(true)
        .timeout(std::time::Duration::from_millis(100));

    client.sync(sync_settings).await?;
    Ok(())
}
