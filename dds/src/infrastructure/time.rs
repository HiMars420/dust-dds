#[derive(PartialOrd, PartialEq, Eq, Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Duration {
    sec: i32,
    nanosec: u32,
}

impl Duration {
    pub const fn new(sec: i32, nanosec: u32) -> Self {
        Self { sec, nanosec }
    }

    /// Get a reference to the duration's sec.
    pub fn sec(&self) -> i32 {
        self.sec
    }

    /// Get a reference to the duration's nanosec.
    pub fn nanosec(&self) -> u32 {
        self.nanosec
    }
}

#[derive(Clone, PartialEq, Debug, Copy, PartialOrd, Eq, Ord)]
pub struct Time {
    pub sec: i32,
    pub nanosec: u32,
}

const SEC_IN_NANOSEC: u64 = 1000000000;

impl From<Time> for u64 {
    fn from(value: Time) -> Self {
        (value.sec as u64 * SEC_IN_NANOSEC) + (value.nanosec as u64 as u64)
    }
}

impl From<u64> for Time {
    fn from(value: u64) -> Self {
        let sec = (value / SEC_IN_NANOSEC) as u64;
        let nanosec = (value - sec * SEC_IN_NANOSEC) as u32;
        let sec = sec as i32;
        Self { sec, nanosec }
    }
}

pub const DURATION_INFINITE: Duration = Duration {
    sec: 0x7fffffff,
    nanosec: 0x7fffffff,
};
pub const DURATION_ZERO: Duration = Duration { sec: 0, nanosec: 0 };
pub const TIME_INVALID: Time = Time {
    sec: -1,
    nanosec: 0xffffffff,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn time_from_u64() {
        let expected_time = Time {
            sec: 11,
            nanosec: 100,
        };
        let value_u64 = 11000000100;
        let time = Time::from(value_u64);
        let time_u64: u64 = time.into();

        assert_eq!(time, expected_time);

        assert_eq!(value_u64, time_u64);
    }
}
