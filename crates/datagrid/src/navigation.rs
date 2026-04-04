use crate::types::CellCoord;

/// Navigation key for 2D grid movement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GridNavKey {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    CtrlHome,
    CtrlEnd,
}

/// Navigate a 2D grid from `current` position using the given `key`.
///
/// Returns the new cell coordinate, clamped to grid bounds.
/// This is a pure function — no Dioxus dependency — fully testable.
///
/// ## Behavior
/// - Arrow keys move one cell in the given direction, clamped at edges.
/// - `Home`/`End` move to the first/last column in the current row.
/// - `CtrlHome`/`CtrlEnd` move to the first/last cell in the entire grid.
pub fn navigate_grid(
    row_count: usize,
    col_count: usize,
    current: CellCoord,
    key: GridNavKey,
) -> CellCoord {
    if row_count == 0 || col_count == 0 {
        return CellCoord::default();
    }

    let max_row = row_count.saturating_sub(1);
    let max_col = col_count.saturating_sub(1);

    match key {
        GridNavKey::Up => CellCoord {
            row_idx: current.row_idx.saturating_sub(1),
            col_idx: current.col_idx,
        },
        GridNavKey::Down => CellCoord {
            row_idx: current.row_idx.saturating_add(1).min(max_row),
            col_idx: current.col_idx,
        },
        GridNavKey::Left => CellCoord {
            row_idx: current.row_idx,
            col_idx: current.col_idx.saturating_sub(1),
        },
        GridNavKey::Right => CellCoord {
            row_idx: current.row_idx,
            col_idx: current.col_idx.saturating_add(1).min(max_col),
        },
        GridNavKey::Home => CellCoord {
            row_idx: current.row_idx,
            col_idx: 0,
        },
        GridNavKey::End => CellCoord {
            row_idx: current.row_idx,
            col_idx: max_col,
        },
        GridNavKey::CtrlHome => CellCoord {
            row_idx: 0,
            col_idx: 0,
        },
        GridNavKey::CtrlEnd => CellCoord {
            row_idx: max_row,
            col_idx: max_col,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn navigate_down_clamps_at_bottom() {
        let result = navigate_grid(
            3,
            4,
            CellCoord {
                row_idx: 2,
                col_idx: 1,
            },
            GridNavKey::Down,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 2,
                col_idx: 1
            }
        );
    }

    #[test]
    fn navigate_up_clamps_at_top() {
        let result = navigate_grid(
            3,
            4,
            CellCoord {
                row_idx: 0,
                col_idx: 1,
            },
            GridNavKey::Up,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 0,
                col_idx: 1
            }
        );
    }

    #[test]
    fn navigate_right_clamps_at_edge() {
        let result = navigate_grid(
            3,
            4,
            CellCoord {
                row_idx: 1,
                col_idx: 3,
            },
            GridNavKey::Right,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 1,
                col_idx: 3
            }
        );
    }

    #[test]
    fn navigate_left_clamps_at_edge() {
        let result = navigate_grid(
            3,
            4,
            CellCoord {
                row_idx: 1,
                col_idx: 0,
            },
            GridNavKey::Left,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 1,
                col_idx: 0
            }
        );
    }

    #[test]
    fn navigate_normal_movement() {
        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 2,
                col_idx: 2,
            },
            GridNavKey::Down,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 3,
                col_idx: 2
            }
        );

        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 2,
                col_idx: 2,
            },
            GridNavKey::Up,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 1,
                col_idx: 2
            }
        );

        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 2,
                col_idx: 2,
            },
            GridNavKey::Right,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 2,
                col_idx: 3
            }
        );

        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 2,
                col_idx: 2,
            },
            GridNavKey::Left,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 2,
                col_idx: 1
            }
        );
    }

    #[test]
    fn navigate_home_end() {
        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 2,
                col_idx: 3,
            },
            GridNavKey::Home,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 2,
                col_idx: 0
            }
        );

        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 2,
                col_idx: 1,
            },
            GridNavKey::End,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 2,
                col_idx: 4
            }
        );
    }

    #[test]
    fn navigate_ctrl_home_end() {
        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 3,
                col_idx: 3,
            },
            GridNavKey::CtrlHome,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 0,
                col_idx: 0
            }
        );

        let result = navigate_grid(
            5,
            5,
            CellCoord {
                row_idx: 1,
                col_idx: 1,
            },
            GridNavKey::CtrlEnd,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 4,
                col_idx: 4
            }
        );
    }

    #[test]
    fn navigate_empty_grid() {
        let result = navigate_grid(
            0,
            0,
            CellCoord {
                row_idx: 0,
                col_idx: 0,
            },
            GridNavKey::Down,
        );
        assert_eq!(result, CellCoord::default());
    }

    #[test]
    fn navigate_single_cell_grid() {
        let result = navigate_grid(
            1,
            1,
            CellCoord {
                row_idx: 0,
                col_idx: 0,
            },
            GridNavKey::Down,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 0,
                col_idx: 0
            }
        );

        let result = navigate_grid(
            1,
            1,
            CellCoord {
                row_idx: 0,
                col_idx: 0,
            },
            GridNavKey::Right,
        );
        assert_eq!(
            result,
            CellCoord {
                row_idx: 0,
                col_idx: 0
            }
        );
    }
}
