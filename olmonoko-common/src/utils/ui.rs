use std::collections::BTreeSet;

/// Used to arrange overlapping events side by side
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Arrangement {
    pub lane: u8,

    /// visible size is calculated to be 1/width
    pub width: u8,
}
impl Default for Arrangement {
    fn default() -> Self {
        Self { lane: 0, width: 1 }
    }
}

pub fn arrange(starts_at: &[i64], durations: &[i32]) -> Vec<Arrangement> {
    let size = starts_at.len();
    assert_eq!(size, durations.len());

    let mut enumerated: Vec<_> = durations.iter().enumerate().collect();
    enumerated.sort_by_key(|t| t.1 * -1000 + t.0 as i32);
    let sorted_by_duration = enumerated.into_iter().map(|t| t.0);

    let mut arrangements: Vec<Arrangement> = vec![Arrangement::default(); size];
    let mut seen: Vec<(i64, i64, usize)> = vec![];
    for i in sorted_by_duration {
        let start = starts_at[i];
        let duration = durations[i];
        let end = start + duration as i64;

        let mut intersecting = vec![];
        let mut occupied_lanes = BTreeSet::new();
        let mut width = 1;
        let mut lane = 0;
        for si in 0..seen.len() {
            let s = seen[si];
            let seen_s = s.0;
            let seen_e = s.1;
            if start < seen_e && end > seen_s {
                intersecting.push(s);
                occupied_lanes.insert(arrangements[s.2].lane);
                width += 1;
            }
        }
        for s in intersecting {
            arrangements[s.2].width = width;
        }
        for l in 0..width {
            if !occupied_lanes.contains(&l) {
                lane = l;
            }
        }
        seen.push((start, end, i));
        arrangements[i].width = width;
        arrangements[i].lane = lane;
    }
    return arrangements;
}

#[cfg(test)]
pub mod tests {
    use crate::utils::ui::Arrangement;

    #[test]
    fn arrange_sanity() {
        let starts_at = [1, 2, 3, 10];
        let durations = [1, 1, 1, 10];
        let arrangement = super::arrange(&starts_at, &durations);
        let default_arr = Arrangement::default();
        assert_eq!(arrangement, [default_arr; 4]);
    }

    #[test]
    fn arrange_two_side_by_side() {
        let starts_at = [1, 1, 3, 10];
        let durations = [1, 1, 1, 10];
        let arrangement = super::arrange(&starts_at, &durations);
        let default_arr = Arrangement::default();
        assert_eq!(
            arrangement,
            [
                Arrangement { lane: 0, width: 2 },
                Arrangement { lane: 1, width: 2 },
                default_arr,
                default_arr
            ]
        );
    }

    #[test]
    fn arrange_three_side_by_side() {
        let starts_at = [1, 1, 1, 10];
        let durations = [1, 2, 1, 10];
        let arrangement = super::arrange(&starts_at, &durations);
        let default_arr = Arrangement::default();
        assert_eq!(
            arrangement,
            [
                Arrangement { lane: 1, width: 3 }, // longer events get preferential treatment
                Arrangement { lane: 0, width: 3 },
                Arrangement { lane: 2, width: 3 },
                default_arr
            ]
        );
    }
}
