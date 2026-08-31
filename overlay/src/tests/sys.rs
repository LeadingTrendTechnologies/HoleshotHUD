use super::shm_publishes;

#[test]
fn seqlock_delta_is_half_the_seq_step() {
    assert_eq!(shm_publishes(0, 140), 70);
    assert_eq!(shm_publishes(10, 10), 0);
    assert_eq!(shm_publishes(u32::MAX - 1, 2), 2);
}
