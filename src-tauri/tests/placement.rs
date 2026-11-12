//! T028 — popup placement: corners, two monitors, widths 380/720, mixed scale, work areas.

use verso_lib::platform::{Point, Rect, Size};
use verso_lib::window_manager::{
    place_popup, PlacementInput, EDGE_MARGIN, PLACEMENT_GAP, POPUP_EXPANDED_WIDTH, POPUP_HEIGHT,
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
        height: POPUP_HEIGHT,
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
        area.bottom() - 10.0 - PLACEMENT_GAP - POPUP_HEIGHT
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
    for width in [POPUP_WIDTH, POPUP_EXPANDED_WIDTH] {
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
                position.y + POPUP_HEIGHT <= area.bottom() - EDGE_MARGIN + 0.001,
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
