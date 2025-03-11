use bevy::prelude::*;
use bevy_rcon::{RconPlayer, RconPlayers, RconPlugin, DbRconBannedPlayer};
use bevy_inspector_egui::quick::WorldInspectorPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        WorldInspectorPlugin::new(),
        RconPlugin,
    ));
    app.register_type::<RconPlayer>();
    app.register_type::<DbRconBannedPlayer>();
    app.add_systems(Startup, setup_players);
    app.run();
}

fn setup_players(mut rcon_players: ResMut<RconPlayers>) {
    rcon_players.players.push(RconPlayer {
        unique_id: "steam_123".to_string(),
        name: "Player1".to_string(),
    });
    rcon_players.players.push(RconPlayer {
        unique_id: "steam_456".to_string(),
        name: "Player2".to_string(),
    });
    rcon_players.players.push(RconPlayer {
        unique_id: "steam_789".to_string(),
        name: "Player3".to_string(),
    });
}
