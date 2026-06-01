use crate::{NetQueue, Paddle};

/// Ingest queued network inputs and apply them as each paddle's target Y.
pub fn ingest_inputs(paddles: &mut [Paddle], net_queue: &mut NetQueue) {
    for (player_id, y_pos) in net_queue.inputs.drain(..) {
        for paddle in paddles.iter_mut() {
            if paddle.player_id == player_id {
                // Clamp to the valid center range for a 4.0-tall paddle in a 24.0 arena.
                paddle.target_y = y_pos.clamp(2.0, 22.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PlayerId;

    #[test]
    fn test_input_applied_to_correct_paddle() {
        let mut paddles = vec![
            Paddle::new(PlayerId(0), 12.0),
            Paddle::new(PlayerId(1), 12.0),
        ];
        let mut net_queue = NetQueue::new();
        net_queue.push_input(PlayerId(0), 5.0);
        net_queue.push_input(PlayerId(1), 18.0);

        ingest_inputs(&mut paddles, &mut net_queue);

        assert_eq!(paddles[0].target_y, 5.0);
        assert_eq!(paddles[1].target_y, 18.0);
    }

    #[test]
    fn test_input_queue_cleared_after_processing() {
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        let mut net_queue = NetQueue::new();
        net_queue.push_input(PlayerId(0), 10.0);
        assert_eq!(net_queue.inputs.len(), 1);

        ingest_inputs(&mut paddles, &mut net_queue);

        assert_eq!(net_queue.inputs.len(), 0, "Input queue should be cleared");
    }

    #[test]
    fn test_multiple_inputs_for_same_player() {
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        let mut net_queue = NetQueue::new();
        net_queue.push_input(PlayerId(0), 5.0);
        net_queue.push_input(PlayerId(0), 15.0);
        net_queue.push_input(PlayerId(0), 8.0); // last wins

        ingest_inputs(&mut paddles, &mut net_queue);

        assert_eq!(
            paddles[0].target_y, 8.0,
            "Last input target should be applied"
        );
    }

    #[test]
    fn test_clamping() {
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        let mut net_queue = NetQueue::new();

        net_queue.push_input(PlayerId(0), -100.0);
        ingest_inputs(&mut paddles, &mut net_queue);
        assert_eq!(paddles[0].target_y, 2.0, "Should clamp to min");

        net_queue.push_input(PlayerId(0), 100.0);
        ingest_inputs(&mut paddles, &mut net_queue);
        assert_eq!(paddles[0].target_y, 22.0, "Should clamp to max");
    }

    #[test]
    fn test_no_panic_when_no_paddles() {
        let mut paddles: Vec<Paddle> = Vec::new();
        let mut net_queue = NetQueue::new();
        net_queue.push_input(PlayerId(0), 10.0);
        ingest_inputs(&mut paddles, &mut net_queue);
    }

    #[test]
    fn test_no_panic_when_no_inputs() {
        let mut paddles = vec![Paddle::new(PlayerId(0), 12.0)];
        let mut net_queue = NetQueue::new();
        ingest_inputs(&mut paddles, &mut net_queue);
    }
}
