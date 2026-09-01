use std::sync::{Mutex, OnceLock};

static MARKS: OnceLock<Mutex<Vec<i32>>> = OnceLock::new();

fn marks() -> &'static Mutex<Vec<i32>> {
    MARKS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn set_presence_marks(nums: &[i32]) {
    if let Ok(mut g) = marks().lock() {
        g.clear();
        g.extend_from_slice(nums);
    }
}

pub fn presence_has(race_num: i32) -> bool {
    if race_num <= 0 {
        return false;
    }
    marks()
        .lock()
        .map(|g| g.contains(&race_num))
        .unwrap_or(false)
}
