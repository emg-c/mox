use mox_format::Event;

/// 편집마다 이전 이벤트 목록을 통째로 보관한다. 6만 개 기준 단계당 약 2~3MB.
pub struct History {
    undo: Vec<Vec<Event>>,
    redo: Vec<Vec<Event>>,
    cap: usize,
}

impl History {
    pub const DEFAULT_CAP: usize = 50;

    pub fn new(cap: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            cap,
        }
    }

    /// 편집을 실행하고, 실제로 바뀌었을 때만 이력에 남긴다.
    pub fn edit<R>(&mut self, events: &mut Vec<Event>, f: impl FnOnce(&mut Vec<Event>) -> R) -> R {
        let before = events.clone();
        let r = f(events);
        if *events != before {
            self.undo.push(before);
            if self.undo.len() > self.cap {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        r
    }

    pub fn undo(&mut self, events: &mut Vec<Event>) -> bool {
        match self.undo.pop() {
            Some(prev) => {
                self.redo.push(std::mem::replace(events, prev));
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self, events: &mut Vec<Event>) -> bool {
        match self.redo.pop() {
            Some(next) => {
                self.undo.push(std::mem::replace(events, next));
                true
            }
            None => false,
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use crate::{remove_moves, scale_delays, Sel};
    use mox_format::KeyCode;
    use proptest::prelude::*;

    #[test]
    fn undo_and_redo_restore_states() {
        let original = vec![mv(0, 0, 0), kd(100, KeyCode::KeyA)];
        let mut evs = original.clone();
        let mut h = History::new(History::DEFAULT_CAP);
        h.edit(&mut evs, remove_moves);
        let after = evs.clone();
        assert!(h.undo(&mut evs));
        assert_eq!(evs, original);
        assert!(h.redo(&mut evs));
        assert_eq!(evs, after);
        assert!(!h.redo(&mut evs));
    }

    #[test]
    fn no_op_edit_is_not_recorded() {
        let mut evs = vec![kd(0, KeyCode::KeyA)];
        let mut h = History::new(10);
        h.edit(&mut evs, remove_moves);
        assert!(!h.can_undo());
    }

    #[test]
    fn new_edit_clears_redo() {
        let mut evs = vec![mv(0, 0, 0), kd(100, KeyCode::KeyA)];
        let mut h = History::new(10);
        h.edit(&mut evs, remove_moves);
        h.undo(&mut evs);
        h.edit(&mut evs, |e| scale_delays(e, Sel::All, 2.0).unwrap());
        assert!(!h.can_redo());
    }

    #[test]
    fn keeps_only_cap_steps() {
        let mut evs = vec![mv(1, 0, 0)];
        let mut h = History::new(3);
        for _ in 0..5 {
            h.edit(&mut evs, |e| scale_delays(e, Sel::All, 2.0).unwrap());
        }
        let mut undone = 0;
        while h.undo(&mut evs) {
            undone += 1;
        }
        assert_eq!(undone, 3);
        assert_eq!(delays(&evs), vec![4]); // 1→2→4→8→16→32 에서 세 번 되돌림
    }

    proptest! {
        #[test]
        fn undo_after_any_edit_is_identity(ds in proptest::collection::vec(0u64..10_000, 0..40), k in 0.0f64..5.0) {
            let original: Vec<_> = ds.iter().enumerate().map(|(i, &d)| mv(d, i as i32, 0)).collect();
            let mut evs = original.clone();
            let mut h = History::new(5);
            h.edit(&mut evs, |e| scale_delays(e, Sel::All, k).unwrap());
            h.edit(&mut evs, crate::thin_moves_half);
            while h.undo(&mut evs) {}
            prop_assert_eq!(evs, original);
        }
    }
}
