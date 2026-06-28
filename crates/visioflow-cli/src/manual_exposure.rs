//! Manual exposure adjustment via preview window arrow keys.

use std::collections::HashSet;

use minifb::{Key, Window};
use visioflow_core::MANUAL_EV_STEP;

/// Adjust `manual_ev` when Up/Down arrows are pressed (0.5 EV per step).
///
/// Returns true when the exposure changed this frame.
pub fn adjust_manual_ev_on_arrow_keys(
    window: &Window,
    manual_ev: &mut f32,
    keys_held: &mut HashSet<Key>,
) -> bool {
    let mut changed = false;

    if apply_arrow_step(window, Key::Up, MANUAL_EV_STEP, manual_ev, keys_held) {
        changed = true;
    }
    if apply_arrow_step(window, Key::Down, -MANUAL_EV_STEP, manual_ev, keys_held) {
        changed = true;
    }

    changed
}

fn apply_arrow_step(
    window: &Window,
    key: Key,
    delta: f32,
    manual_ev: &mut f32,
    keys_held: &mut HashSet<Key>,
) -> bool {
    let down = window.is_key_down(key);
    let newly_pressed = down && !keys_held.contains(&key);

    if down {
        keys_held.insert(key);
    } else {
        keys_held.remove(&key);
    }

    if newly_pressed {
        *manual_ev += delta;
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_ev_step_is_half_stop() {
        assert!((MANUAL_EV_STEP - 0.5).abs() < f32::EPSILON);
    }
}
