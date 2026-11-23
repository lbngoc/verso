//! T028 — popup placement: corners, two monitors, widths 380/720, mixed scale, work areas.

use verso_lib::platform::{Point, Rect, Size};
use verso_lib::window_manager::{
    place_popup, PlacementInput, EDGE_MARGIN, PLACEMENT_GAP, POPUP_MAX_WIDTH, POPUP_RESULT_HEIGHT,
    POPUP_WIDTH,
};

fn work_area() -> Rect {
    Rect {
        x: 0.0,
        y: 25.0,
        width: 1512.0,
        height: 960.0,
    }
}

fn size(width: f64) -> Size {
    Size {
        width,
        height: POPUP_RESULT_HEIGHT,
    }
}

#[test]
fn places_below_right_of_the_cursor_in_the_middle() {
    let position = place_popup(&PlacementInput {
        cursor: Point { x: 500.0, y: 400.0 },
        work_area: work_area(),
        size: size(POPUP_WIDTH),
        scale_factor: 2.0,
        gap: PLACEMENT_GAP,
    });
    assert_eq!(position.x, 500.0 + PLACEMENT_GAP);
    assert_eq!(position.y, 400.0 + PLACEMENT_GAP);
}

#[test]
fn near_the_bottom_edge_it_flips_above_the_cursor() {
    let area = work_area();
    let position = place_popup(&PlacementInput {
        cursor: Point {
            x: 400.0,
            y: area.bottom() - 10.0,
        },
        work_area: area,
        size: size(POPUP_WIDTH),
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });
    assert_eq!(
        position.y,
        area.bottom() - 10.0 - PLACEMENT_GAP - POPUP_RESULT_HEIGHT
    );
    assert!(position.y >= area.y + EDGE_MARGIN);
}

#[test]
fn near_the_right_edge_it_flips_to_the_left_and_clamps() {
    let area = work_area();
    let position = place_popup(&PlacementInput {
        cursor: Point {
            x: area.right() - 5.0,
            y: 300.0,
        },
        work_area: area,
        size: size(POPUP_WIDTH),
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });
    assert!(position.x + POPUP_WIDTH <= area.right() - EDGE_MARGIN + 0.001);
    assert!(position.x >= area.x + EDGE_MARGIN);
}

#[test]
fn all_four_corners_stay_inside_the_work_area() {
    let area = work_area();
    let corners = [
        Point {
            x: area.x + 1.0,
            y: area.y + 1.0,
        },
        Point {
            x: area.right() - 1.0,
            y: area.y + 1.0,
        },
        Point {
            x: area.x + 1.0,
            y: area.bottom() - 1.0,
        },
        Point {
            x: area.right() - 1.0,
            y: area.bottom() - 1.0,
        },
    ];
    for width in [POPUP_WIDTH, POPUP_MAX_WIDTH] {
        for cursor in corners {
            let position = place_popup(&PlacementInput {
                cursor,
                work_area: area,
                size: size(width),
                scale_factor: 1.0,
                gap: PLACEMENT_GAP,
            });
            assert!(
                position.x >= area.x + EDGE_MARGIN - 0.001,
                "x {position:x?} left of work area for width {width}"
            );
            assert!(
                position.x + width <= area.right() - EDGE_MARGIN + 0.001,
                "width {width} overflows on the right"
            );
            assert!(position.y >= area.y + EDGE_MARGIN - 0.001);
            assert!(
                position.y + POPUP_RESULT_HEIGHT <= area.bottom() - EDGE_MARGIN + 0.001,
                "height overflows at the bottom"
            );
        }
    }
}

#[test]
fn a_second_monitor_uses_its_own_work_area() {
    let second = Rect {
        x: 1512.0,
        y: 0.0,
        width: 1920.0,
        height: 1080.0,
    };
    let cursor = Point {
        x: second.x + 100.0,
        y: second.y + 100.0,
    };
    let position = place_popup(&PlacementInput {
        cursor,
        work_area: second,
        size: size(POPUP_WIDTH),
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });
    assert!(position.x >= second.x);
    assert!(position.x < second.right());
}

#[test]
fn mixed_scale_factors_do_not_change_logical_placement() {
    let area = work_area();
    let base = PlacementInput {
        cursor: Point { x: 700.0, y: 500.0 },
        work_area: area,
        size: size(POPUP_WIDTH),
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    };
    let retina = PlacementInput {
        scale_factor: 2.0,
        ..base
    };
    assert_eq!(place_popup(&base), place_popup(&retina));
}

#[test]
fn the_menu_bar_is_excluded_by_the_work_area() {
    let area = Rect {
        x: 0.0,
        y: 25.0,
        width: 1440.0,
        height: 875.0,
    };
    let position = place_popup(&PlacementInput {
        cursor: Point { x: 100.0, y: 30.0 },
        work_area: area,
        size: size(POPUP_WIDTH),
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    });
    assert!(position.y >= area.y + EDGE_MARGIN);
}

// ---- spec 004 (T025): the popup next to a screen region ----

use verso_lib::window_manager::place_near_region;

fn region(x: f64, y: f64, width: f64, height: f64) -> Rect {
    Rect {
        x,
        y,
        width,
        height,
    }
}

fn inside(area: Rect, position: Point, size: Size) -> bool {
    position.x >= area.x + EDGE_MARGIN - 0.001
        && position.y >= area.y + EDGE_MARGIN - 0.001
        && position.x + size.width <= area.right() - EDGE_MARGIN + 0.001
        && position.y + size.height <= area.bottom() - EDGE_MARGIN + 0.001
}

#[test]
fn a_region_gets_the_popup_below_it_aligned_to_its_left_edge() {
    let area = work_area();
    let target = region(300.0, 200.0, 400.0, 60.0);
    let popup = size(POPUP_WIDTH);
    let position = place_near_region(target, area, popup, PLACEMENT_GAP);
    assert_eq!(position.x, 300.0);
    assert_eq!(position.y, target.bottom() + PLACEMENT_GAP);
}

#[test]
fn with_no_room_below_the_popup_goes_above() {
    let area = work_area();
    let target = region(300.0, area.bottom() - 90.0, 400.0, 60.0);
    let popup = size(POPUP_WIDTH);
    let position = place_near_region(target, area, popup, PLACEMENT_GAP);
    assert_eq!(position.y, target.y - PLACEMENT_GAP - popup.height);
    assert!(inside(area, position, popup));
}

#[test]
fn a_tall_region_gets_the_popup_to_the_right_then_the_left() {
    let area = work_area();
    let popup = size(POPUP_WIDTH);
    // Fills the height: neither above nor below fits, there is room on the right.
    let target = region(100.0, area.y, 300.0, area.height);
    let position = place_near_region(target, area, popup, PLACEMENT_GAP);
    assert_eq!(position.x, target.right() + PLACEMENT_GAP);
    assert!(inside(area, position, popup));
    // Against the right edge: the left side is used.
    let target = region(area.right() - 320.0, area.y, 300.0, area.height);
    let position = place_near_region(target, area, popup, PLACEMENT_GAP);
    assert_eq!(position.x, target.x - PLACEMENT_GAP - popup.width);
    assert!(inside(area, position, popup));
}

#[test]
fn a_region_filling_the_screen_gets_the_popup_inside_its_lower_part() {
    // Revision 2: the result panel sits over the bottom of a region that fills the screen, where
    // a result is read, not over its top.
    let area = work_area();
    let popup = size(POPUP_WIDTH);
    let target = region(area.x, area.y, area.width, area.height);
    let position = place_near_region(target, area, popup, PLACEMENT_GAP);
    assert!(inside(area, position, popup));
    assert!(position.y + popup.height >= area.bottom() - EDGE_MARGIN - 0.001);
}

#[test]
fn the_popup_always_stays_inside_the_work_area() {
    let area = work_area();
    let popup = size(POPUP_MAX_WIDTH);
    for (x, y) in [
        (0.0, 25.0),
        (1400.0, 25.0),
        (0.0, 900.0),
        (1450.0, 940.0),
        (700.0, 500.0),
    ] {
        let target = region(x, y, 60.0, 30.0);
        let position = place_near_region(target, area, popup, PLACEMENT_GAP);
        assert!(
            inside(area, position, popup),
            "region at {x},{y} -> {position:?}"
        );
    }
}

#[test]
fn a_work_area_with_a_negative_origin_is_handled() {
    // A monitor to the left of the primary one.
    let area = Rect {
        x: -1440.0,
        y: 25.0,
        width: 1440.0,
        height: 875.0,
    };
    let popup = size(POPUP_WIDTH);
    let target = region(-1000.0, 300.0, 300.0, 40.0);
    let position = place_near_region(target, area, popup, PLACEMENT_GAP);
    assert_eq!(position.x, -1000.0);
    assert_eq!(position.y, target.bottom() + PLACEMENT_GAP);
    let near_edge = region(-1430.0, 300.0, 100.0, 40.0);
    let position = place_near_region(near_edge, area, popup, PLACEMENT_GAP);
    assert!(inside(area, position, popup));
}

// ---- spec 004: the update window's height follows its content ----

#[test]
fn the_update_window_height_is_clamped_to_220_and_560() {
    use verso_lib::window_manager::clamp_update_height;
    assert_eq!(clamp_update_height(100.0), 220.0);
    assert_eq!(clamp_update_height(300.0), 300.0);
    assert_eq!(clamp_update_height(900.0), 560.0);
    assert_eq!(clamp_update_height(f64::NAN), 220.0);
}

// ---- spec 004 Revision 2 (T068): the panel is placed by how it was opened ----

use verso_lib::window_manager::{
    frame_in_slot, place_panel, popup_slot, PanelAnchor, QUICK_HEIGHT, QUICK_WIDTH,
};

fn panel() -> Size {
    Size {
        width: QUICK_WIDTH,
        height: QUICK_HEIGHT,
    }
}

/// A macOS screen with a notch-height menu bar (work area top at 38).
fn notch_area() -> Rect {
    region(0.0, 38.0, 1512.0, 944.0)
}

#[test]
fn a_tray_icon_in_the_menu_bar_puts_the_panel_just_below_it_centred() {
    let area = notch_area();
    let icon = region(1000.0, 0.0, 24.0, 24.0);
    let position = place_panel(PanelAnchor::TrayIcon(icon), area, panel(), PLACEMENT_GAP);
    assert_eq!(position.y, area.y + PLACEMENT_GAP);
    assert_eq!(position.x, 1012.0 - QUICK_WIDTH / 2.0);
}

#[test]
fn a_tray_icon_at_the_right_edge_is_clamped_inside() {
    let area = notch_area();
    let icon = region(area.right() - 30.0, 0.0, 24.0, 24.0);
    let position = place_panel(PanelAnchor::TrayIcon(icon), area, panel(), PLACEMENT_GAP);
    assert_eq!(position.x, area.right() - QUICK_WIDTH - EDGE_MARGIN);
}

#[test]
fn a_tray_icon_on_a_bottom_taskbar_puts_the_panel_just_above_it() {
    let area = region(0.0, 0.0, 1920.0, 1040.0);
    let icon = region(1200.0, 1040.0, 24.0, 40.0);
    let position = place_panel(PanelAnchor::TrayIcon(icon), area, panel(), PLACEMENT_GAP);
    assert_eq!(position.y, area.bottom() - PLACEMENT_GAP - QUICK_HEIGHT);
    assert_eq!(position.x, 1212.0 - QUICK_WIDTH / 2.0);
}

#[test]
fn a_tray_icon_on_a_side_taskbar_puts_the_panel_beside_it_centred() {
    let left = region(48.0, 0.0, 1872.0, 1080.0);
    let icon = region(8.0, 500.0, 32.0, 32.0);
    let position = place_panel(PanelAnchor::TrayIcon(icon), left, panel(), PLACEMENT_GAP);
    assert_eq!(position.x, left.x + PLACEMENT_GAP);
    assert_eq!(position.y, 516.0 - QUICK_HEIGHT / 2.0);

    let right = region(0.0, 0.0, 1872.0, 1080.0);
    let icon = region(1880.0, 20.0, 32.0, 32.0);
    let position = place_panel(PanelAnchor::TrayIcon(icon), right, panel(), PLACEMENT_GAP);
    assert_eq!(position.x, right.right() - PLACEMENT_GAP - QUICK_WIDTH);
    assert!(position.y >= right.y + EDGE_MARGIN, "clamped at the top");
}

#[test]
fn the_shortcut_puts_the_panel_centred_in_the_upper_third() {
    let area = notch_area();
    let position = place_panel(PanelAnchor::Shortcut, area, panel(), PLACEMENT_GAP);
    assert_eq!(position.x, (area.width - QUICK_WIDTH) / 2.0);
    assert_eq!(position.y, area.y + (area.height - QUICK_HEIGHT) / 3.0);
}

#[test]
fn the_shortcut_on_a_monitor_with_a_negative_origin_stays_on_it() {
    let area = region(-1440.0, 25.0, 1440.0, 875.0);
    let position = place_panel(PanelAnchor::Shortcut, area, panel(), PLACEMENT_GAP);
    assert!(inside(area, position, panel()));
    assert_eq!(position.x, area.x + (area.width - QUICK_WIDTH) / 2.0);
}

#[test]
fn a_region_anchor_follows_the_region_rules() {
    let area = notch_area();
    let target = region(300.0, 200.0, 200.0, 60.0);
    let position = place_panel(PanelAnchor::Region(target), area, panel(), PLACEMENT_GAP);
    assert_eq!(position.x, 300.0);
    assert_eq!(position.y, target.bottom() + PLACEMENT_GAP);

    let low = region(300.0, 880.0, 200.0, 60.0);
    let position = place_panel(PanelAnchor::Region(low), area, panel(), PLACEMENT_GAP);
    assert_eq!(position.y, low.y - PLACEMENT_GAP - QUICK_HEIGHT);

    let filling = region(area.x, area.y, area.width, area.height);
    let position = place_panel(PanelAnchor::Region(filling), area, panel(), PLACEMENT_GAP);
    assert!(inside(area, position, panel()));
    assert!(position.y + QUICK_HEIGHT >= area.bottom() - EDGE_MARGIN - 0.001);
}

// ---- T080: the popup's pill and its result share one corner ----

fn slot_input(cursor: Point, area: Rect) -> PlacementInput {
    PlacementInput {
        cursor,
        work_area: area,
        size: size(POPUP_WIDTH),
        scale_factor: 1.0,
        gap: PLACEMENT_GAP,
    }
}

#[test]
fn the_pill_and_the_result_share_the_top_left_when_placed_below() {
    let area = work_area();
    let slot = popup_slot(&slot_input(Point { x: 500.0, y: 300.0 }, area));
    assert!(!slot.grow_up);
    let pill = frame_in_slot(slot, verso_lib::window_manager::popup_pill_size(), area);
    let result = frame_in_slot(slot, size(POPUP_WIDTH), area);
    assert_eq!((pill.x, pill.y), (result.x, result.y));
    assert_eq!(pill.y, 300.0 + PLACEMENT_GAP);
}

#[test]
fn the_pill_and_the_result_share_the_bottom_left_when_placed_above() {
    let area = work_area();
    let slot = popup_slot(&slot_input(Point { x: 500.0, y: 940.0 }, area));
    assert!(slot.grow_up);
    let pill = frame_in_slot(slot, verso_lib::window_manager::popup_pill_size(), area);
    let result = frame_in_slot(slot, size(POPUP_WIDTH), area);
    assert_eq!(pill.x, result.x);
    assert!((pill.bottom() - result.bottom()).abs() < 0.001);
    assert!(result.y >= area.y + EDGE_MARGIN - 0.001);
}
