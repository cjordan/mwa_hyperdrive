// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::path::{Path, PathBuf};

use permissions::is_readable;
use regex::{Regex, RegexBuilder};
use thiserror::Error;

use crate::glob::{get_all_matches_from_glob, GlobError};

lazy_static::lazy_static! {
    static ref RE_METAFITS: Regex =
        RegexBuilder::new(r"\.metafits$")
            .case_insensitive(true).build().unwrap();

    // gpubox files should not be renamed in any way! This includes the case of
    // the letters in the filename. mwalib should complain if this is not the
    // case.
    static ref RE_GPUBOX: Regex =
        RegexBuilder::new(r".*gpubox.*\.fits$")
            .case_insensitive(false).build().unwrap();

    static ref RE_MWAX: Regex =
        RegexBuilder::new(r"\d{10}_\d{8}(.)?\d{6}_ch\d{3}_\d{3}\.fits$")
            .case_insensitive(false).build().unwrap();

    static ref RE_MWAF: Regex =
        RegexBuilder::new(r"\.mwaf$")
                .case_insensitive(true).build().unwrap();

    static ref RE_MS: Regex =
        RegexBuilder::new(r"\.ms$")
            .case_insensitive(true).build().unwrap();

    static ref RE_UVFITS: Regex =
        RegexBuilder::new(r"\.uvfits$")
            .case_insensitive(true).build().unwrap();
}

#[derive(Debug)]
pub struct InputDataTypes {
    pub metafits: Option<PathBuf>,
    pub gpuboxes: Option<Vec<PathBuf>>,
    pub mwafs: Option<Vec<PathBuf>>,
    pub ms: Option<PathBuf>,
    pub uvfits: Option<Vec<PathBuf>>,
}

// The same as `InputDataTypes`, but all types are allowed to be multiples. This
// makes coding easier.
#[derive(Debug, Default)]
struct InputDataTypesTemp {
    metafits: Vec<PathBuf>,
    gpuboxes: Vec<PathBuf>,
    mwafs: Vec<PathBuf>,
    ms: Vec<PathBuf>,
    uvfits: Vec<PathBuf>,
}

impl InputDataTypes {
    /// From an input collection of filename or glob strings, disentangle the
    /// file types and populate [InputDataTypes].
    pub fn new(files: &[String]) -> Result<Self, InputFileError> {
        let mut temp = InputDataTypesTemp::default();

        for file in files.iter().map(|f| f.as_str()) {
            file_checker(&mut temp, &file)?;
        }

        if temp.metafits.len() > 1 {
            return Err(InputFileError::MultipleMetafits(
                temp.metafits
                    .into_iter()
                    .map(|pb| pb.display().to_string())
                    .collect(),
            ));
        }
        if temp.ms.len() > 1 {
            return Err(InputFileError::MultipleMeasurementSets(
                temp.ms
                    .into_iter()
                    .map(|pb| pb.display().to_string())
                    .collect(),
            ));
        }

        Ok(Self {
            metafits: temp.metafits.first().cloned(),
            gpuboxes: if temp.gpuboxes.is_empty() {
                None
            } else {
                Some(temp.gpuboxes)
            },
            mwafs: if temp.mwafs.is_empty() {
                None
            } else {
                Some(temp.mwafs)
            },
            ms: temp.ms.first().cloned(),
            uvfits: if temp.uvfits.is_empty() {
                None
            } else {
                Some(temp.uvfits)
            },
        })
    }
}

fn exists_and_is_readable(file: &Path) -> Result<(), InputFileError> {
    if !file.exists() {
        return Err(InputFileError::DoesNotExist(file.display().to_string()));
    }
    match is_readable(file) {
        Ok(true) => (),
        Ok(false) => return Err(InputFileError::CouldNotRead(file.display().to_string())),
        Err(e) => return Err(InputFileError::IO(file.display().to_string(), e)),
    }

    Ok(())
}

// Given a file (as a string), check it exists and is readable, then determine
// what type it is, and add it to the provided file types struct. If the file
// string doesn't exist, then check if it's a glob string, and act recursively
// on the glob results.
fn file_checker(file_types: &mut InputDataTypesTemp, file: &str) -> Result<(), InputFileError> {
    let file_pb = PathBuf::from(file);
    // Is this a file, and is it readable?
    match exists_and_is_readable(&file_pb) {
        Ok(_) => (),

        // If this string isn't a file, maybe it's a glob.
        Err(InputFileError::DoesNotExist(f)) => {
            match get_all_matches_from_glob(file) {
                Ok(glob_results) => {
                    // Iterate over all glob results, adding them to the file
                    // types.
                    for pb in glob_results {
                        file_checker(file_types, pb.display().to_string().as_str())?;
                    }
                }

                // If there were no glob matches, then just return the original
                // error (the file does not exist).
                Err(GlobError::NoMatches { .. }) => return Err(InputFileError::DoesNotExist(f)),

                // Propagate all other errors.
                Err(e) => return Err(InputFileError::from(e)),
            }
        }

        // Propagate all other errors.
        Err(e) => return Err(e),
    };
    match (
        RE_METAFITS.is_match(file),
        RE_GPUBOX.is_match(file),
        RE_MWAX.is_match(file),
        RE_MWAF.is_match(file),
        RE_MS.is_match(file),
        RE_UVFITS.is_match(file),
    ) {
        (true, _, _, _, _, _) => file_types.metafits.push(file_pb),
        (_, true, _, _, _, _) => file_types.gpuboxes.push(file_pb),
        (_, _, true, _, _, _) => file_types.gpuboxes.push(file_pb),
        (_, _, _, true, _, _) => file_types.mwafs.push(file_pb),
        (_, _, _, _, true, _) => file_types.ms.push(file_pb),
        (_, _, _, _, _, true) => file_types.uvfits.push(file_pb),
        _ => return Err(InputFileError::NotRecognised(file.to_string())),
    }

    Ok(())
}

#[derive(Debug, Error)]
pub enum InputFileError {
    #[error("Specified file does not exist: {0}")]
    DoesNotExist(String),

    #[error("Could not read specified file: {0}")]
    CouldNotRead(String),

    #[error("The specified file '{0}' was not a recognised file type.")]
    NotRecognised(String),

    #[error("Multiple metafits files were specified: {0:?}")]
    MultipleMetafits(Vec<String>),

    #[error("Multiple measurement sets were specified: {0:?}")]
    MultipleMeasurementSets(Vec<String>),

    #[error("{0}")]
    Glob(#[from] GlobError),

    #[error("IO error when attempting to read file '{0}': {1}")]
    IO(String, std::io::Error),
}