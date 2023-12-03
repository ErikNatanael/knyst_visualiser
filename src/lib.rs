use std::sync::mpsc::Receiver;

use bevy::{core::Zeroable, prelude::*, window::PrimaryWindow};
use knyst::{controller::KnystCommands, graph::NodeId, inspection::GraphInspection, knyst};
use rand::{thread_rng, Rng};

pub fn init_knyst_visualiser() {
    println!("Hello, world!");
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_non_send_resource(KnystData::new())
        .add_systems(Startup, setup)
        .add_systems(Update, update_inspection)
        .add_systems(Update, draw_edges)
        .add_systems(Update, move_nodes)
        // .add_systems(Update, update_velocities)
        .add_systems(Update, apply_velocities)
        .add_systems(Update, move_camera_mouse)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let font = asset_server.load("fonts/Terminess (TTF) Bold Nerd Font Complete.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 30.0,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment::Center;
    // 2d camera
    commands.spawn((Camera2dBundle::default(), GameCamera));
}

#[derive(Component)]
struct GameCamera;

#[derive(Component)]
struct Node {
    id: NodeId,
    num_inputs: usize,
    num_outputs: usize,
    edge_acceleration: f32,
}
#[derive(Component)]
struct Graph(u64);

#[derive(Component)]
struct GraphOutputs {
    num_outputs: usize,
    graph_id: u64,
}

#[derive(Component)]
struct NodeEdge {
    from_entity: Entity,
    to_entity: Entity,
    from_channel_index: usize,
    to_channel_index: usize,
}
#[derive(Component)]
struct Velocity(Vec2);

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

fn node_height(num_inputs: usize, num_outputs: usize) -> f32 {
    15. * num_inputs.max(num_outputs).max(1) as f32
}

fn update_inspection(
    mut commands: Commands,
    mut knyst_data: NonSendMut<KnystData>,
    mut graph_query: Query<(&mut Graph)>,
    mut node_query: Query<(&mut Node, Entity)>,
    mut q_graph_output: Query<(&mut GraphOutputs, Entity)>,
    asset_server: Res<AssetServer>,
) {
    let mut new_inspection_available = false;
    if let Some(recv) = &mut knyst_data.next_receiver {
        if let Ok(new_inspection) = recv.try_recv() {
            knyst_data.latest_inspection = new_inspection;
            new_inspection_available = true;
        }
    } else {
        let inspection_receiver = knyst().request_inspection();
        knyst_data.next_receiver = Some(inspection_receiver);
    }
    let font = asset_server.load("fonts/Terminess (TTF) Bold Nerd Font Complete.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 20.0,
        color: Color::WHITE,
    };
    let text_alignment = TextAlignment::Center;
    let mut rng = thread_rng();
    let mut edges_to_add = vec![];
    let mut new_nodes = vec![];
    if new_inspection_available {
        info!("New inspeciton available");
        if q_graph_output.is_empty() {
            let graph_outputs = knyst_data.latest_inspection.num_outputs;
            // Spawn a new node
            let parent = commands
                .spawn((
                    SpatialBundle {
                        transform: Transform::from_translation(Vec3::new(500., 0., 0.)),
                        ..Default::default()
                    },
                    Velocity(Vec2::ZERO),
                    GraphOutputs {
                        num_outputs: graph_outputs,
                        graph_id: knyst_data.latest_inspection.graph_id,
                    },
                ))
                .id();
            let mut children = Vec::new();
            let rect = commands
                .spawn((SpriteBundle {
                    sprite: Sprite {
                        color: Color::rgb(0.0, 0.25, 0.75),
                        custom_size: Some(Vec2::new(160.0, 15. * graph_outputs as f32)),
                        ..default()
                    },
                    transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
                    ..default()
                },))
                .id();
            let name = commands
                .spawn((Text2dBundle {
                    text: Text::from_section("GraphOutputs", text_style.clone())
                        .with_alignment(text_alignment),
                    ..default()
                },))
                .id();
            children.push(rect);
            children.push(name);
            commands.entity(parent).push_children(&children);
            for edge in &knyst_data.latest_inspection.graph_output_input_edges {
                edges_to_add.push((*edge, parent));
            }
        }
        for node in &knyst_data.latest_inspection.nodes {
            if !node_query.iter().any(|n| n.0.id == node.address) {
                let size = node.input_channels.len().max(node.output_channels.len()) + 1;
                // Spawn a new node
                let parent = commands
                    .spawn((
                        SpatialBundle {
                            transform: Transform::from_translation(Vec3::new(
                                rng.gen_range(-300.0..300.),
                                rng.gen_range(-300.0..300.0),
                                0.,
                            )),
                            ..Default::default()
                        },
                        Velocity(Vec2::ZERO),
                        Node {
                            id: node.address,
                            num_inputs: node.input_channels.len(),
                            num_outputs: node.output_channels.len(),
                            edge_acceleration: 1.0,
                        },
                    ))
                    .id();
                let mut children = Vec::new();
                let rect = commands
                    .spawn((SpriteBundle {
                        sprite: Sprite {
                            color: Color::rgb(0.0, 0.25, 0.75),
                            custom_size: Some(Vec2::new(
                                160.0,
                                node_height(node.input_channels.len(), node.output_channels.len()),
                            )),
                            ..default()
                        },
                        transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
                        ..default()
                    },))
                    .id();
                let name_text = match node.name.as_str() {
                    "MulGen" => "*",
                    "PowfGen" => "^",
                    _ => &node.name,
                };
                let name = commands
                    .spawn((Text2dBundle {
                        text: Text::from_section(name_text, text_style.clone())
                            .with_alignment(text_alignment),
                        transform: Transform::from_xyz(0.0, 0.0, 10.),
                        ..default()
                    },))
                    .id();
                children.push(name);
                children.push(rect);
                let channel_text_style = TextStyle {
                    font: font.clone(),
                    font_size: 10.0,
                    color: Color::WHITE,
                };
                for (i, input) in node.input_channels.iter().enumerate() {
                    let text = commands
                        .spawn((Text2dBundle {
                            text: Text::from_section(input, channel_text_style.clone())
                                .with_alignment(TextAlignment::Left),
                            transform: Transform::from_xyz(-80., i as f32 * -15., 0.),
                            ..default()
                        },))
                        .id();
                    children.push(text);
                }
                for (i, output) in node.output_channels.iter().enumerate() {
                    let text = commands
                        .spawn((Text2dBundle {
                            text: Text::from_section(output, channel_text_style.clone())
                                .with_alignment(TextAlignment::Right),
                            transform: Transform::from_xyz(80., i as f32 * -15., 0.),
                            ..default()
                        },))
                        .id();
                    children.push(text);
                }
                commands.entity(parent).push_children(&children);
                for edge in &node.input_edges {
                    edges_to_add.push((*edge, parent));
                }
                new_nodes.push((parent, node.address));
            }
        }
        for (edge, sink_node_entity) in edges_to_add {
            // Find the source entity
            let source = match edge.source {
                knyst::inspection::EdgeSource::Node(index) => {
                    let id = knyst_data.latest_inspection.nodes[index].address;
                    if let Some((_node, entity)) =
                        node_query.iter().find(|(node, _ent)| node.id == id)
                    {
                        warn!("Found entity");
                        Some(entity)
                    } else {
                        if let Some((entity, _id)) =
                            new_nodes.iter().find(|(_entity, nid)| *nid == id)
                        {
                            warn!("Found entity among new");
                            Some(*entity)
                        } else {
                            warn!("Unable to find entity");
                            None
                        }
                    }
                }
                knyst::inspection::EdgeSource::Graph => todo!(),
            };

            if let Some(source) = source {
                commands.spawn(NodeEdge {
                    from_entity: source,
                    to_entity: sink_node_entity,
                    from_channel_index: edge.from_index,
                    to_channel_index: edge.to_index,
                });
            }
        }

        for g in &mut graph_query {}
    }
}

fn draw_edges(
    mut gizmos: Gizmos,
    node_query: Query<(&Node, &Transform)>,
    graph_output_query: Query<(&GraphOutputs, &Transform)>,
    edge_query: Query<(&NodeEdge)>,
) {
    for edge in edge_query.iter() {
        let NodeEdge {
            from_entity,
            to_entity,
            from_channel_index,
            to_channel_index,
        } = edge;
        let origin_pos = if let Ok((_, from_node_transform)) = node_query.get(*from_entity) {
            from_node_transform.translation.xy()
                + Vec2::new(80., *from_channel_index as f32 * -15.0 + 7.5)
        } else {
            Vec2::new(0.0, 0.0)
        };

        let end_pos = if let Ok((_, to_node_transform)) = node_query.get(*to_entity) {
            to_node_transform.translation.xy()
                + Vec2::new(-80., *to_channel_index as f32 * -15.0 + 7.5)
        } else {
            if let Ok((_, to_graph_transform)) = graph_output_query.get(*to_entity) {
                to_graph_transform.translation.xy()
                    + Vec2::new(-80., *to_channel_index as f32 * -15.0 + 7.5)
            } else {
                Vec2::new(0., 0.)
            }
        };
        gizmos.line_2d(origin_pos, end_pos, Color::RED);
    }
}

fn update_velocities(
    mut node_query: Query<(&mut Node, &Transform, &mut Velocity)>,
    mut q_graph_outputs: Query<(&mut GraphOutputs, &Transform)>,
    edge_query: Query<&NodeEdge>,
) {
    for (_node, _transform, mut vel) in node_query.iter_mut() {
        vel.0 *= Vec2::splat(0.5);
    }
    for edge in edge_query.iter() {
        let NodeEdge {
            from_entity,
            to_entity,
            from_channel_index,
            to_channel_index,
        } = edge;
        let origin_pos = if let Ok((_, from_node_transform, _vel)) = node_query.get(*from_entity) {
            from_node_transform.translation.xy()
        } else {
            Vec2::new(0.0, 0.0)
        };

        let mut to_node_transform_pos = None;
        if let Ok((node, to_node_transform, mut vel)) = node_query.get_mut(*to_entity) {
            let end_pos = to_node_transform.translation.xy();
            to_node_transform_pos = Some(to_node_transform.translation.xy());
            let diff = origin_pos - end_pos
                + Vec2::new(
                    180.,
                    -15. * *to_channel_index as f32 + (node.num_inputs as f32 * 15. * 0.5),
                );
            if diff.length_squared() > 60. {
                vel.0 += diff.clamp_length_max(50.) * 0.15 * node.edge_acceleration;
            } else {
                vel.0 -= diff * 0.15 * node.edge_acceleration;
            }
        }
        if let Ok((mut node, from_node_transform, mut vel)) = node_query.get_mut(*from_entity) {
            if let Some(origin_pos) = to_node_transform_pos {
                let end_pos = from_node_transform.translation.xy();
                let diff =
                    origin_pos - end_pos + Vec2::new(-180., -15. * *from_channel_index as f32);
                if diff.length_squared() > 60. {
                    vel.0 += diff.clamp_length_max(50.) * 0.15;
                } else {
                    vel.0 -= diff * 0.15;
                }
            }
            if node.edge_acceleration > 0.1 {
                node.edge_acceleration *= 0.95;
            }
        }
    }
    // Move away from other nodes
    // This force should be weaker than the force from edges when the edges are far apart.
    let mut combinations = node_query.iter_combinations_mut();
    while let Some([mut n0, mut n1]) = combinations.fetch_next() {
        let diff = n0.1.translation.xy() - n1.1.translation.xy();
        if diff.length_squared() < 100. {
            let vel = diff.normalize();
            n0.2 .0 += vel * 4.0;
            n1.2 .0 += vel * 4.0;
        }
    }
}

fn apply_velocities(mut node_query: Query<(&Node, &mut Transform, &Velocity)>) {
    for (_node, mut transform, vel) in node_query.iter_mut() {
        transform.translation += Vec3::from((vel.0, 0.));
    }
}

fn move_nodes(
    mut node_query: Query<(&mut Node, &mut Transform), Without<GraphOutputs>>,
    q_graph_outputs: Query<(&Transform, Entity, &GraphOutputs)>,
    edge_query: Query<&NodeEdge>,
) {
    let mut node_entities_in_current_column = vec![];
    let mut node_entities_to_put_in_the_next_column = vec![];
    // First find the inputs to the GraphOutputs and to nodes that are unconnected to the graph outputs.
    // TODO: unconnected nodes
    let column_size = 180.;
    let row_gap = 10.;
    let Ok((go_transform, go_entity, go)) = q_graph_outputs.get_single() else {
        return;
    };
    node_entities_in_current_column.push(go_entity);
    let start_y = go_transform.translation.y;
    let mut current_column = go_transform.translation.x - column_size;
    let mut previous_column_height = node_height(go.num_outputs, go.num_outputs);
    while !node_entities_in_current_column.is_empty() {
        for edge in edge_query.iter() {
            if node_entities_in_current_column.contains(&edge.to_entity) {
                if !node_entities_to_put_in_the_next_column.contains(&edge.from_entity) {
                    node_entities_to_put_in_the_next_column.push(edge.from_entity);
                }
            }
        }
        // 
        let mut y = 0.;
        for node_entity in &node_entities_to_put_in_the_next_column {
            // Move
            if let Ok((node, mut transform)) = node_query.get_mut(*node_entity) {
                transform.translation.x = current_column;
                transform.translation.y = y + start_y;
                y -= node_height(node.num_inputs, node.num_outputs) + row_gap;
            }
        }
        if previous_column_height.abs() > (y).abs() {
            y = 0.;
            let row_gap = (previous_column_height.abs() - (y).abs()) / (node_entities_to_put_in_the_next_column.len()+1) as f32;
        for node_entity in &node_entities_to_put_in_the_next_column {
            // Move
            if let Ok((node, mut transform)) = node_query.get_mut(*node_entity) {
                transform.translation.x = current_column;
                transform.translation.y = y + start_y;
                y -= node_height(node.num_inputs, node.num_outputs) + row_gap;
            }
        }

        }
        current_column -= column_size;
        std::mem::swap(
            &mut node_entities_in_current_column,
            &mut node_entities_to_put_in_the_next_column,
        );
        node_entities_to_put_in_the_next_column.clear();
        previous_column_height = y;
    }

    // Recursively add any inputs to the current column until there are no more
}

fn move_camera_mouse(
    q_windows: Query<&Window, With<PrimaryWindow>>,
    mut q_camera: Query<&mut Transform, With<GameCamera>>,
) {
    // Games typically only have one window (the primary window)
    if let Some(position) = q_windows.single().cursor_position() {
        println!("Cursor is inside the primary window, at {:?}", position);
        let window_height = q_windows.single().height();
        let window_width = q_windows.single().width();
        let margin = 50.;
        let speed = 5.;
        let mut vel = Vec2::zeroed();
        if position.x < margin {
            vel += Vec2::new(speed * -1., 0.0);
        }
        if position.y < margin {
            vel += Vec2::new(0.0, speed);
        }
        if position.x > window_width - margin {
            vel += Vec2::new(speed, 0.0);
        }
        if position.y > window_height - margin {
            vel += Vec2::new(0.0, speed * -1.);
        }
        q_camera.single_mut().translation += Vec3::from((vel, 0.0));
    } else {
        println!("Cursor is not in the game window.");
    }
}
