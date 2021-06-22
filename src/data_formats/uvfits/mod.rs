// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Code to handle reading from and writing to uvfits files.

mod error;
mod read;
mod write;

pub(crate) use error::*;
pub(crate) use read::*;
pub(crate) use write::*;

use std::ffi::CString;

use hifitime::Epoch;

/// From a `hifitime` [Epoch], get a formatted date string with the hours,
/// minutes and seconds set to 0.
fn get_truncated_date_string(epoch: &Epoch) -> String {
    let (year, month, day, _, _, _, _) = epoch.as_gregorian_utc();
    format!(
        "{year}-{month}-{day}T00:00:00.0",
        year = year,
        month = month,
        day = day
    )
}

/// Helper function to convert strings into pointers of C strings.
fn rust_strings_to_c_strings<T: AsRef<str>>(
    strings: &[T],
) -> Result<Vec<*mut i8>, std::ffi::NulError> {
    let mut c_strings = Vec::with_capacity(strings.len());
    for s in strings {
        let rust_str = s.as_ref().to_owned();
        let c_str = CString::new(rust_str)?;
        c_strings.push(c_str.into_raw());
    }
    Ok(c_strings)
}

/// Encode a baseline into the uvfits format. Use the miriad convention to
/// handle more than 255 antennas (up to 2048). This is backwards compatible
/// with the standard UVFITS convention. Antenna indices start at 1.
// Shamelessly copied from the RTS, originally written by Randall Wayth.
fn encode_uvfits_baseline(ant1: usize, ant2: usize) -> usize {
    if ant2 > 255 {
        ant1 * 2048 + ant2 + 65_536
    } else {
        ant1 * 256 + ant2
    }
}

/// Decode a uvfits baseline into the antennas that formed it. Antenna indices
/// start at 1.
fn decode_uvfits_baseline(bl: usize) -> (usize, usize) {
    if bl < 65_535 {
        let ant2 = bl % 256;
        let ant1 = (bl - ant2) / 256;
        (ant1, ant2)
    } else {
        let ant2 = (bl - 65_536) % 2048;
        let ant1 = (bl - ant2 - 65_536) / 2048;
        (ant1, ant2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_truncated_date_str() {
        let mjd = 56580.575370370374;
        let mjd_seconds = mjd * 24.0 * 3600.0;
        // The number of seconds between 1858-11-17T00:00:00 (MJD epoch) and
        // 1900-01-01T00:00:00 (TAI epoch) is 1297728000.
        let epoch_diff = 1297728000.0;
        let epoch = Epoch::from_tai_seconds(mjd_seconds - epoch_diff);
        assert_eq!(get_truncated_date_string(&epoch), "2013-10-15T00:00:00.0");
    }

    #[test]
    fn test_encode_uvfits_baseline() {
        assert_eq!(encode_uvfits_baseline(1, 1), 257);
        // TODO: Test the other part of the if statement.
    }

    #[test]
    fn test_decode_uvfits_baseline() {
        assert_eq!(decode_uvfits_baseline(257), (1, 1));
        // TODO: Test the other part of the if statement.
    }
}