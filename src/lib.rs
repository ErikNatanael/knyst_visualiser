use std::sync::mpsc::Receiver;

use bevy::prelude::*;
use knyst::{commands, controller::KnystCommands, graph::NodeId, inspection::GraphInspection};

pub fn init_knyst_visualiser() {
    println!("Hello, world!");
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(KnystData::new())
        .add_systems(Startup, setup)
        .add_systems(Update, update_inspection)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/Terminess (TTF) Bold Nerd Font Complete.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment::Center;
    // 2d camera
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        Text2dBundle {
            text: Text::from_section("Graph", text_style.clone()).with_alignment(text_alignment),
            ..default()
        },
        Graph(0),
    ));
    // Rectangle
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::rgb(0.25, 0.25, 0.75),
            custom_size: Some(Vec2::new(50.0, 100.0)),
            ..default()
        },
        transform: Transform::from_translation(Vec3::new(-50., 0., 0.)),
        ..default()
    });
}

#[derive(Component)]
struct Node(NodeId);
#[derive(Component)]
struct Graph(u64);

struct KnystData {
    latest_inspection: GraphInspection,
    next_receiver: Option<Receiver<GraphInspection>>,
}
impl KnystData {
    fn new() -> Self {
        Self {
            latest_inspection: GraphInspection::empty(),
            next_receiver: None,
        }
    }
}

fn update_inspection(mut knyst_data: NonSendMut<KnystData>) {
    let mut new_inspection_available = false;
    if let Some(recv) = &mut knyst_data.next_receiver {
        if let Ok(new_inspection) = recv.try_recv() {
            knyst_data.latest_inspection = new_inspection;
            new_inspection_available = true;
        }
    } else {
        let inspection_receiver = commands().request_inspection();
        knyst_data.next_receiver = Some(inspection_receiver);
    }

    if new_inspection_available {
        println!("New inspeciton available");
    }
}
