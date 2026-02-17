use matrix_sdk::{
    Client,
    room::Room,
    ruma::events::room::member::{MembershipState, OriginalSyncRoomMemberEvent},
};

pub async fn on_room_invite(event: OriginalSyncRoomMemberEvent, room: Room, client: Client) {
    if event.content.membership != MembershipState::Invite {
        return;
    }

    let me = client.user_id().unwrap().to_owned();
    if event.state_key != me.as_str() {
        return;
    }

    let room_id = room.room_id().to_owned();
    println!("Invited to room: {}", room_id);

    if let Err(err) = room.join().await {
        eprintln!("Failed to join {}: {:?}", room_id, err);
    } else {
        println!("Joined room {}", room_id);
    }
}
