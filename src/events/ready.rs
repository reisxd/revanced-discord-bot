use chrono::Utc;
use tracing::trace;

use super::*;
use crate::db::model::Muted;
use crate::utils::bot::get_data_lock;
use crate::utils::moderation::queue_unmute_member;

pub async fn load_muted_members(ctx: &serenity::Context, _: &serenity::Ready) {
    let data = get_data_lock(ctx).await;
    let data = data.read().await;
    let database = &data.database;
    let mute_role_id = data.configuration.read().await.general.mute.role;
    let mut pending_unmutes = data.pending_unmutes.write().await;

    let mut cursor = database
        .find::<Muted>(
            "muted",
            Muted {
                ..Default::default()
            }
            .into(),
            None,
        )
        .await
        .unwrap();

    let http_ref = &ctx.http;

    while cursor.advance().await.unwrap() {
        let current: Muted = cursor.deserialize_current().unwrap();
        let guild_id = current.guild_id.unwrap().parse::<u64>().unwrap();
        let member_id = current.user_id.unwrap().parse::<u64>().unwrap();

        if let Ok(member) = http_ref
            .get_guild(guild_id)
            .await
            .unwrap()
            .member(http_ref, member_id)
            .await
        {
            let amount_left =
                std::cmp::max(current.expires.unwrap() as i64 - Utc::now().timestamp(), 0);

            pending_unmutes.insert(
                member.user.id.0,
                queue_unmute_member(
                    ctx,
                    database,
                    &member,
                    mute_role_id,
                    amount_left as u64, // i64 as u64 is handled properly here
                ),
            );
        } else {
            trace!("Failed to find member {} in guild {}", member_id, guild_id);
        }
    }
}
