// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::ops::Sub;

use hifitime::{Duration, Epoch, Unit};
use itertools::Itertools;

use mwa_hyperdrive_common::{hifitime, itertools, lazy_static};

lazy_static::lazy_static! {
    static ref DURATION_MAX: Duration = Duration::from_f64(f64::MAX, Unit::Second);
}

pub fn guess_interval<T, O>(elements: &[T]) -> Option<O>
where
    T: Sub<Output = O> + Copy,
    O: PartialOrd,
{
    elements
        .iter()
        .tuple_windows()
        .map(|(&a, &b)| b.sub(a))
        .min_by(|a, b| a.partial_cmp(b).unwrap())
}

pub fn guess_time_res(timestamps: Vec<Epoch>) -> Option<Duration> {
    guess_interval(&timestamps)
}

pub fn guess_freq_res(frequencies: Vec<f64>) -> Option<f64> {
    guess_interval(&frequencies)
}
