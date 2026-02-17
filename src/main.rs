use anyhow::Result;
use catbot::invite::on_room_invite;
use catbot::models::cat::Cat;
use catbot::models::joke::Joke;
use dotenv::dotenv;
use matrix_sdk::room::Room;
use matrix_sdk::ruma::events::room::message::OriginalSyncRoomMessageEvent;
use matrix_sdk::ruma::events::room::message::{
    ImageMessageEventContent, MessageType, RoomMessageEventContent,
};
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

    // --- FIX IS HERE ---
    // We create a specific clone to be moved into the closure
    let client_handle = client.clone();

    client.add_event_handler(move |ev: OriginalSyncRoomMessageEvent, room: Room| {
        let client = client_handle.clone();
        async move {
            let body = ev.content.body();

            if body.starts_with("!cat") {
                let cat_url = match reqwest::get("https://api.thecatapi.com/v1/images/search").await
                {
                    Ok(resp) => match resp.json::<Vec<Cat>>().await {
                        Ok(list) if !list.is_empty() => list[0].url.clone(),
                        _ => {
                            eprintln!("Failed to parse cat API response");
                            return;
                        }
                    },
                    Err(err) => {
                        eprintln!("Failed to fetch cat API: {:?}", err);
                        return;
                    }
                };

                let bytes_vec = match reqwest::get(&cat_url).await {
                    Ok(resp) => match resp.bytes().await {
                        Ok(b) => b.to_vec(),
                        Err(_) => {
                            eprintln!("Failed to read image bytes");
                            return;
                        }
                    },
                    Err(_) => {
                        eprintln!("Failed to download cat image");
                        return;
                    }
                };

                // Use the captured 'client' here
                let upload_result = match client
                    .media()
                    .upload(&mime::IMAGE_JPEG, bytes_vec.clone(), None)
                    .await
                {
                    Ok(upload) => upload,
                    Err(err) => {
                        eprintln!("Media upload failed: {:?}", err);
                        return;
                    }
                };

                let mxc_uri = upload_result.content_uri;

                let image_content =
                    ImageMessageEventContent::plain("cat.jpg".to_string(), mxc_uri.clone());

                let message = RoomMessageEventContent::new(MessageType::Image(image_content));

                if let Err(err) = room.send(message).await {
                    eprintln!("Failed to send cat image: {:?}", err);
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
        }
    });

    // This now works because `client` was not moved above
    client.add_event_handler(on_room_invite);

    let sync_settings = SyncSettings::default()
        .ignore_timeout_on_first_sync(true)
        .timeout(std::time::Duration::from_millis(100));

    client.sync(sync_settings).await?;
    Ok(())
}
