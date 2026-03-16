use bevy::prelude::*;
use carbonthrone::zone::ZoneKind;

use super::{StateUiRoot, panel_bg, text_font, white_text};
use crate::resources::{GameSessionRes, MinimapOpen};
use crate::state::AppState;

pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Exploration), spawn_minimap)
            .add_systems(
                Update,
                (toggle_minimap, update_minimap).run_if(in_state(AppState::Exploration)),
            );
    }
}

#[derive(Component)]
struct MinimapRoot;

#[derive(Component)]
struct MinimapZoneBox(ZoneKind);

// Layout constants
const CELL_W: f32 = 48.0;
const CELL_H: f32 = 40.0;
const BOX_W: f32 = 36.0;
const BOX_H: f32 = 18.0;
const PAD: f32 = 8.0;
const LINE_THICKNESS: f32 = 3.0;

// Zone grid positions (col, row)
fn zone_grid_pos(zone: ZoneKind) -> (f32, f32) {
    match zone {
        ZoneKind::RelayArray => (2.0, 0.0),
        ZoneKind::ExcavationSite => (0.0, 1.0),
        ZoneKind::StationExterior => (2.0, 1.0),
        ZoneKind::CommandDeck => (1.0, 2.0),
        ZoneKind::DockingBay => (2.0, 2.0),
        ZoneKind::MilitaryAnnex => (3.0, 2.0),
        ZoneKind::ResearchWing => (1.0, 3.0),
        ZoneKind::SystemsCore => (2.0, 3.0),
        ZoneKind::MedicalBay => (3.0, 3.0),
        ZoneKind::Hallway => (0.0, 0.0), // not shown on minimap
    }
}

fn zone_label(zone: ZoneKind) -> &'static str {
    match zone {
        ZoneKind::RelayArray => "REL",
        ZoneKind::ExcavationSite => "EXC",
        ZoneKind::StationExterior => "EXT",
        ZoneKind::CommandDeck => "CMD",
        ZoneKind::DockingBay => "DOC",
        ZoneKind::MilitaryAnnex => "MIL",
        ZoneKind::ResearchWing => "RES",
        ZoneKind::SystemsCore => "SYS",
        ZoneKind::MedicalBay => "MED",
        ZoneKind::Hallway => "---",
    }
}

const SHOWN_ZONES: &[ZoneKind] = &[
    ZoneKind::RelayArray,
    ZoneKind::ExcavationSite,
    ZoneKind::StationExterior,
    ZoneKind::CommandDeck,
    ZoneKind::DockingBay,
    ZoneKind::MilitaryAnnex,
    ZoneKind::ResearchWing,
    ZoneKind::SystemsCore,
    ZoneKind::MedicalBay,
];

// Connections as (from, to)
const CONNECTIONS: &[(ZoneKind, ZoneKind)] = &[
    (ZoneKind::RelayArray, ZoneKind::StationExterior),
    (ZoneKind::ExcavationSite, ZoneKind::StationExterior),
    (ZoneKind::StationExterior, ZoneKind::DockingBay),
    (ZoneKind::CommandDeck, ZoneKind::DockingBay),
    (ZoneKind::DockingBay, ZoneKind::MilitaryAnnex),
    (ZoneKind::CommandDeck, ZoneKind::ResearchWing),
    (ZoneKind::DockingBay, ZoneKind::SystemsCore),
    (ZoneKind::MilitaryAnnex, ZoneKind::MedicalBay),
    (ZoneKind::ResearchWing, ZoneKind::SystemsCore),
    (ZoneKind::SystemsCore, ZoneKind::MedicalBay),
];

const DIM_GRAY: Color = Color::srgba(0.3, 0.3, 0.35, 0.9);
const ACTIVE_COLOR: Color = Color::srgb(0.4, 0.8, 1.0);
const LINE_COLOR: Color = Color::srgba(0.5, 0.5, 0.55, 0.7);

fn box_center(col: f32, row: f32) -> (f32, f32) {
    let x = PAD + col * CELL_W + CELL_W / 2.0;
    let y = PAD + row * CELL_H + CELL_H / 2.0;
    (x, y)
}

fn spawn_minimap(mut commands: Commands, minimap_open: Res<MinimapOpen>) {
    let panel_w = PAD * 2.0 + 4.0 * CELL_W;
    let panel_h = PAD * 2.0 + 4.0 * CELL_H;

    let visibility = if minimap_open.0 {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };

    commands
        .spawn((
            StateUiRoot,
            MinimapRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(8.0),
                right: Val::Px(8.0),
                width: Val::Px(panel_w),
                height: Val::Px(panel_h),
                ..default()
            },
            panel_bg(),
            visibility,
        ))
        .with_children(|parent| {
            // Spawn connection lines
            for (a, b) in CONNECTIONS {
                let (ax, ay) = box_center(zone_grid_pos(*a).0, zone_grid_pos(*a).1);
                let (bx, by) = box_center(zone_grid_pos(*b).0, zone_grid_pos(*b).1);

                let (left, top, width, height) = if (ax - bx).abs() > (ay - by).abs() {
                    // horizontal line
                    let lx = ax.min(bx);
                    let rx = ax.max(bx);
                    let cy = (ay + by) / 2.0;
                    (lx, cy - LINE_THICKNESS / 2.0, rx - lx, LINE_THICKNESS)
                } else {
                    // vertical line
                    let ty = ay.min(by);
                    let by2 = ay.max(by);
                    let cx = (ax + bx) / 2.0;
                    (cx - LINE_THICKNESS / 2.0, ty, LINE_THICKNESS, by2 - ty)
                };

                parent.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(left),
                        top: Val::Px(top),
                        width: Val::Px(width),
                        height: Val::Px(height),
                        ..default()
                    },
                    BackgroundColor(LINE_COLOR),
                ));
            }

            // Spawn zone boxes
            for &zone in SHOWN_ZONES {
                let (col, row) = zone_grid_pos(zone);
                let (cx, cy) = box_center(col, row);
                let left = cx - BOX_W / 2.0;
                let top = cy - BOX_H / 2.0;

                parent
                    .spawn((
                        MinimapZoneBox(zone),
                        Node {
                            position_type: PositionType::Absolute,
                            left: Val::Px(left),
                            top: Val::Px(top),
                            width: Val::Px(BOX_W),
                            height: Val::Px(BOX_H),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(DIM_GRAY),
                    ))
                    .with_children(|b| {
                        b.spawn((Text::new(zone_label(zone)), text_font(8.0), white_text()));
                    });
            }
        });
}

fn toggle_minimap(
    keys: Res<ButtonInput<KeyCode>>,
    mut minimap_open: ResMut<MinimapOpen>,
    mut root_q: Query<&mut Visibility, With<MinimapRoot>>,
) {
    if keys.just_pressed(KeyCode::Tab) {
        minimap_open.0 = !minimap_open.0;
        if let Ok(mut vis) = root_q.single_mut() {
            *vis = if minimap_open.0 {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}

fn update_minimap(
    session: Res<GameSessionRes>,
    minimap_open: Res<MinimapOpen>,
    mut zone_q: Query<(&MinimapZoneBox, &mut BackgroundColor)>,
) {
    if !minimap_open.0 {
        return;
    }
    let current = session.current_zone_kind();
    for (MinimapZoneBox(kind), mut bg) in &mut zone_q {
        bg.0 = if current == Some(*kind) {
            ACTIVE_COLOR
        } else {
            DIM_GRAY
        };
    }
}
