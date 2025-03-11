mod template;

use bevy::{prelude::*, utils::HashMap};
use bevy_defer::{AsyncAccess, AsyncWorld};
use bevy_easy_database::{AddDatabaseMapping, DatabasePlugin};
use bevy_webserver::{BevyWebServerPlugin, RouterAppExt};
use maud::{html, Markup};
use serde::{Deserialize, Serialize};
use template::{base_template, TemplateParams};

pub struct RconPlugin;

impl Plugin for RconPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            BevyWebServerPlugin,
            DatabasePlugin,
        ))
        .insert_resource(RconPlayers { players: vec![] })
        .add_database_mapping::<DbRconBannedPlayer>()
        // Routes
        .route("/", axum::routing::get(index))
        .route("/players", axum::routing::get(list_players))
        .route("/ban_list", axum::routing::get(list_bans))
        .route("/ban_player", axum::routing::post(ban_player))
        .route("/unban_player/{id}", axum::routing::post(unban_player));
    }
}

/// An event that is sent to the plugin user when a player is banned.
/// The plugin user can then perform the appropriate action to disconnect the player.
#[derive(Event)]
pub struct RconPlayerBanned {
    pub player: RconPlayer,
}

/// An event that is sent to the plugin user when a player is kicked.
/// The plugin user can then perform the appropriate action to disconnect the player.
#[derive(Event)]
pub struct RconPlayerKicked {
    pub player: RconPlayer,
}

/// A resource that contains the players currently connected to the server.
/// Must be maintained by the plugin user, add and remove players as they connect and disconnect.
/// However, when someone is banned or kicked, the plugin will remove them from the list automatically.
/// And then will send an event to the plugin user to notify them that the player has been removed, 
/// so that they can perform the appropriate action to disconnect the player.
/// See RconPlayerBanned and RconPlayerKicked events for more information.
#[derive(Resource, Default, Reflect)]
pub struct RconPlayers {
    pub players: Vec<RconPlayer>,
}

/// A struct that represents a player connected to the server.
#[derive(Clone, Default, Serialize, Deserialize, PartialEq, Reflect)]
pub struct RconPlayer {
    pub unique_id: String,
    pub name: String,
}

impl std::fmt::Display for RconPlayer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (ID: {})", self.name, self.unique_id)
    }
}

/// A struct that represents a banned player. Contains information about the player at the time they were banned.
#[derive(Component, Clone, Default, Serialize, Deserialize, PartialEq, Reflect)]
pub struct DbRconBannedPlayer {
    pub unique_id: String,
    pub name: String,
}

async fn index() -> axum::response::Html<String> {
    let markup = base_template(TemplateParams {
        tab_title: "RCON Player Management".to_string(),
        game_name: "Game Name".to_string(),
        server_name: "Server Name".to_string(),
        content: html! {
            h3 { "Connected Players" }
            div id="player-list" hx-get="/players" hx-trigger="load" {}
            h3 { "Banned Players" }
            div id="banned-player-list" hx-get="/ban_list" hx-trigger="load" {}
        }
    });

    axum::response::Html(markup.into_string())
}

async fn list_players() -> axum::response::Html<String> {
    let players = AsyncWorld.resource::<RconPlayers>();    
    let players = players.get_mut(|players| {
        players.players.clone()
    }).unwrap();

    let markup = html! {
        div class="player-list" {
            @for player in players {
                (player_item(&player))
            }
        }
    };

    axum::response::Html(markup.into_string())
}

/// A function that returns markup for a player item in the player list.
fn player_item(player: &RconPlayer) -> Markup {
    let is_banned = AsyncWorld
        .query::<&DbRconBannedPlayer>()
        .get_mut(|mut query| {
            query.iter().next()
                .map(|banned| banned.unique_id == player.unique_id)
                .unwrap_or(false)
        })
        .unwrap_or(false);

    if is_banned {
        warn!("You have added a banned player to the player list: {} (ID: {}). Omitting from list. But you should check if a player is banned before adding them to the player list.", player.name, player.unique_id);
    }

    html! {
        @if !is_banned {
            div class="player-item" {
                span { 
                    (player.name) " (ID: " (player.unique_id) ")"
                }

                form 
                    hx-post="/ban_player"
                    hx-target="body"
                    hx-swap="innerHTML"
                {
                    input type="hidden" name="unique_id" value=(player.unique_id);
                    input type="hidden" name="name" value=(player.name);
                    button type="submit" { "Ban" }
                }
            }
        }
    }
}

/// Lists all banned players (database query).
async fn list_bans() -> axum::response::Html<String> {
    let banned_players = AsyncWorld.query::<&DbRconBannedPlayer>();
    let banned_players = banned_players.get_mut(|mut query| -> Vec<DbRconBannedPlayer> {
        let mut players = vec![];
        for player in query.iter() {
            players.push(player.clone());
        }
        players
    }).unwrap_or_default();

    let markup = html! {
        div class="ban-list" {
            @for player in banned_players {
                div class="banned-player" {
                    span { (player.name) " (ID: " (player.unique_id) ")" }
                    button
                        hx-post={"/unban_player/" (player.unique_id)}
                        hx-target="body"
                        hx-swap="innerHTML"
                        { "Unban" }
                }
            }
        }
    };

    axum::response::Html(markup.into_string())
}

/// Adds a player to the banned list (database update).
/// Also removes the player from the player list.
async fn ban_player(
    form: axum::extract::Form<RconPlayer>,
) -> axum::response::Html<String> {
    let id = form.unique_id.clone();
    let name = form.name.clone();

    if id.is_empty() || name.is_empty() {
        warn!("Invalid player data: ID: {}, Name: {}", id, name);
        return index().await;
    }

    AsyncWorld.spawn_bundle(DbRconBannedPlayer {
        unique_id: id.clone(),
        name: name,
    });

    // remove the player from the player list
    if let Err(e) = AsyncWorld.resource::<RconPlayers>().get_mut(|players| {
        players.players.retain(|player| player.unique_id != id);
    }) {
        error!("Failed to remove player from player list: {}", e);
    }

    index().await
}

/// Removes a player from the banned list (database update).
async fn unban_player(
    path: axum::extract::Path<String>,
) -> axum::response::Html<String> {
    
    AsyncWorld.apply_command(move |world: &mut World| {
        let id = path.0.clone();

        let mut banned_players = world.query::<(Entity, &mut DbRconBannedPlayer)>();
        for (entity, banned) in banned_players.iter_mut(world) {
            if banned.unique_id == id {
                world.despawn(entity);
                break;
            }
        }
    });
    index().await
}
