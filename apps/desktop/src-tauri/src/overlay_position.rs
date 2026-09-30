//! Remember the rail position without allowing a removed monitor to hide it.
use crate::DesktopState;
use ort_storage::{EncryptedStore, StorageError};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewWindow};

const SETTING: &str = "overlay.position.v1";

#[derive(Default)]
pub(crate) struct OverlayPositionState(AtomicBool);

#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Position {
    x: i32,
    y: i32,
}

#[derive(Clone, Copy)]
struct WorkArea {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    scale: f64,
}

fn load(store: &EncryptedStore) -> Result<Option<Position>, StorageError> {
    store
        .load_setting(SETTING)?
        .map(|saved| serde_json::from_value(saved.value).map_err(|_| StorageError::InvalidData))
        .transpose()
}

fn save(store: &EncryptedStore, position: Position) -> Result<(), StorageError> {
    let current = store.load_setting(SETTING)?;
    let value = serde_json::to_value(position).map_err(|_| StorageError::InvalidData)?;
    if current.as_ref().is_some_and(|saved| saved.value == value) {
        return Ok(());
    }
    store.save_setting(SETTING, current.map(|saved| saved.revision), &value)?;
    Ok(())
}

pub(crate) fn remember(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("overlay")
        && app
            .state::<OverlayPositionState>()
            .0
            .load(Ordering::Relaxed)
        && let Ok(position) = window.outer_position()
    {
        // Layout preferences must never block closing or storage recovery.
        let _ = app.state::<DesktopState>().with_store(|store| {
            save(
                store,
                Position {
                    x: position.x,
                    y: position.y,
                },
            )
        });
    }
}

fn placement(
    position: Option<Position>,
    areas: &[WorkArea],
) -> Option<(Position, PhysicalSize<u32>)> {
    let area = if let Some(position) = position {
        areas.iter().min_by_key(|area| {
            let x = i64::from(position.x);
            let y = i64::from(position.y);
            let near_x = x.clamp(i64::from(area.x), i64::from(area.x) + i64::from(area.width));
            let near_y = y.clamp(
                i64::from(area.y),
                i64::from(area.y) + i64::from(area.height),
            );
            (x - near_x).abs() + (y - near_y).abs()
        })?
    } else {
        areas.first()?
    };
    let width = crate::physical_dimension(crate::OVERLAY_LOGICAL_WIDTH, area.scale).min(area.width);
    let height =
        crate::physical_dimension(crate::OVERLAY_LOGICAL_HEIGHT, area.scale).min(area.height);
    let position = position.unwrap_or(Position {
        x: area.x,
        y: area
            .y
            .saturating_add(crate::signed_dimension((area.height - height) / 2)),
    });
    let max_x = area
        .x
        .saturating_add(crate::signed_dimension(area.width - width));
    let max_y = area
        .y
        .saturating_add(crate::signed_dimension(area.height - height));
    Some((
        Position {
            x: position.x.clamp(area.x, max_x),
            y: position.y.clamp(area.y, max_y),
        },
        PhysicalSize::new(width, height),
    ))
}

pub(crate) fn restore(window: &WebviewWindow) -> tauri::Result<()> {
    let state = window.state::<OverlayPositionState>();
    let position = if state.0.load(Ordering::Relaxed) {
        window
            .outer_position()
            .ok()
            .map(|p| Position { x: p.x, y: p.y })
    } else {
        window
            .state::<DesktopState>()
            .with_store(load)
            .ok()
            .flatten()
    };
    let mut monitors = window.available_monitors()?;
    if let Some(primary) = window.primary_monitor()? {
        monitors.sort_by_key(|monitor| monitor.position() != primary.position());
    }
    let areas: Vec<_> = monitors
        .iter()
        .filter(|monitor| monitor.work_area().size.width > 0 && monitor.work_area().size.height > 0)
        .map(|monitor| {
            let work = monitor.work_area();
            WorkArea {
                x: work.position.x,
                y: work.position.y,
                width: work.size.width,
                height: work.size.height,
                scale: monitor.scale_factor(),
            }
        })
        .collect();
    if let Some((position, size)) = placement(position, &areas) {
        window.set_size(size)?;
        window.set_position(PhysicalPosition::new(position.x, position.y))?;
        state.0.store(true, Ordering::Relaxed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ort_vault::testing::MemoryDatabaseKeyVault;

    #[test]
    fn position_survives_storage_reopen_and_retains_a_valid_location() {
        let temp = tempfile::TempDir::new().unwrap();
        let vault = MemoryDatabaseKeyVault::new();
        let position = Position { x: 350, y: 20 };
        {
            let store =
                EncryptedStore::open_or_initialize(temp.path(), "position-test", &vault).unwrap();
            save(&store, position).unwrap();
            save(&store, position).unwrap();
            assert_eq!(store.load_setting(SETTING).unwrap().unwrap().revision, 1);
        }
        let store =
            EncryptedStore::open_or_initialize(temp.path(), "position-test", &vault).unwrap();
        let areas = [WorkArea {
            x: 0,
            y: 0,
            width: 1200,
            height: 900,
            scale: 1.0,
        }];
        assert_eq!(
            placement(load(&store).unwrap(), &areas).unwrap().0,
            position
        );
    }

    #[test]
    fn removed_monitor_and_changed_scale_keep_the_rail_visible() {
        let areas = [WorkArea {
            x: 0,
            y: 25,
            width: 1920,
            height: 1055,
            scale: 2.0,
        }];
        let (position, size) = placement(Some(Position { x: -3000, y: 2000 }), &areas).unwrap();
        assert_eq!(position, Position { x: 0, y: 25 });
        assert_eq!(size, PhysicalSize::new(720, 1055));
        let second = [
            WorkArea {
                x: -1600,
                y: 0,
                width: 1600,
                height: 900,
                scale: 1.0,
            },
            areas[0],
        ];
        assert_eq!(
            placement(Some(Position { x: -1200, y: 50 }), &second)
                .unwrap()
                .0,
            Position { x: -1200, y: 50 }
        );
    }
}
