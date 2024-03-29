use std::ops::Range;

use num::Integer;

pub fn map<T: Integer + Copy>(value: T, in_range: Range<T>, out_range: Range<T>) -> T {
    (value - in_range.start) * (out_range.end - out_range.start) / (in_range.end - in_range.start)
        + out_range.start
}
