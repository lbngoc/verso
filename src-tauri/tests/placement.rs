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
fn a_region_filling_the_screen_gets_the_popup_inside_its_top_clamped() {
    let area = work_area();
    let popup = size(POPUP_WIDTH);
    let target = region(area.x, area.y, area.width, area.height);
    let position = place_near_region(target, area, popup, PLACEMENT_GAP);
    assert!(inside(area, position, popup));
    assert!(position.y <= area.y + EDGE_MARGIN + 0.001);
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
