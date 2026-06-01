use crate::{NetQueue, Paddle, PaddleIntent};
use hecs::World;

/// Ingest network inputs and apply by updating paddle targets
pub fn ingest_inputs(world: &mut World, net_queue: &mut NetQueue) {
    // Process all queued inputs
    for (player_id, y_pos) in net_queue.inputs.drain(..) {
        // Find paddle and intent with matching player_id
        for (_entity, (paddle, intent)) in world.query_mut::<(&Paddle, &mut PaddleIntent)>() {
            if paddle.player_id == player_id {
                // Update target, clamped to valid arena range for the center of the paddle
                // Arena height is 24.0, paddle height 4.0.
                // Valid center range: 2.0 to 22.0
                intent.target_y = y_pos.clamp(2.0, 22.0);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{create_paddle, NetQueue, Paddle, PlayerId};

    fn setup_world() -> (hecs::World, NetQueue) {
        (hecs::World::new(), NetQueue::new())
    }

    #[test]
    fn test_input_applied_to_correct_paddle() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, PlayerId(0), 12.0);
        create_paddle(&mut world, PlayerId(1), 12.0);

        // Queue input for player 0
        net_queue.push_input(PlayerId(0), 5.0);
        net_queue.push_input(PlayerId(1), 18.0);

        ingest_inputs(&mut world, &mut net_queue);

        // Verify targets were updated correctly
        let mut paddle_targets = Vec::new();
        for (_entity, (paddle, intent)) in world.query::<(&Paddle, &PaddleIntent)>().iter() {
            paddle_targets.push((paddle.player_id, intent.target_y));
        }
        paddle_targets.sort_by_key(|(id, _)| *id);

        assert_eq!(paddle_targets.len(), 2);
        assert_eq!(paddle_targets[0], (PlayerId(0), 5.0));
        assert_eq!(paddle_targets[1], (PlayerId(1), 18.0));
    }

    #[test]
    fn test_input_queue_cleared_after_processing() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, PlayerId(0), 12.0);

        net_queue.push_input(PlayerId(0), 10.0);
        assert_eq!(net_queue.inputs.len(), 1);

        ingest_inputs(&mut world, &mut net_queue);

        assert_eq!(net_queue.inputs.len(), 0, "Input queue should be cleared");
    }

    #[test]
    fn test_multiple_inputs_for_same_player() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, PlayerId(0), 12.0);

        // Queue multiple inputs (last one should win)
        net_queue.push_input(PlayerId(0), 5.0);
        net_queue.push_input(PlayerId(0), 15.0);
        net_queue.push_input(PlayerId(0), 8.0); // Last

        ingest_inputs(&mut world, &mut net_queue);

        // Last input target should be applied
        for (_entity, (paddle, intent)) in world.query::<(&Paddle, &PaddleIntent)>().iter() {
            if paddle.player_id == PlayerId(0) {
                assert_eq!(intent.target_y, 8.0, "Last input target should be applied");
            }
        }
    }

    #[test]
    fn test_clamping() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, PlayerId(0), 12.0);

        net_queue.push_input(PlayerId(0), -100.0); // Too low
        ingest_inputs(&mut world, &mut net_queue);
        for (_entity, (paddle, intent)) in world.query::<(&Paddle, &PaddleIntent)>().iter() {
            if paddle.player_id == PlayerId(0) {
                assert_eq!(intent.target_y, 2.0, "Should clamp target to min");
            }
        }

        net_queue.push_input(PlayerId(0), 100.0); // Too high
        ingest_inputs(&mut world, &mut net_queue);
        for (_entity, (paddle, intent)) in world.query::<(&Paddle, &PaddleIntent)>().iter() {
            if paddle.player_id == PlayerId(0) {
                assert_eq!(intent.target_y, 22.0, "Should clamp target to max");
            }
        }
    }

    #[test]
    fn test_no_panic_when_no_paddles() {
        let (mut world, mut net_queue) = setup_world();
        net_queue.push_input(PlayerId(0), 10.0);

        // Should not panic
        ingest_inputs(&mut world, &mut net_queue);
    }

    #[test]
    fn test_no_panic_when_no_inputs() {
        let (mut world, mut net_queue) = setup_world();
        create_paddle(&mut world, PlayerId(0), 12.0);

        // Should not panic
        ingest_inputs(&mut world, &mut net_queue);
    }
}
