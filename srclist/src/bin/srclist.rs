// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Handle source list files.
//!
//! The executable generated by this file can verify that hyperdrive can read
//! source files, as well as convert between supported source list formats.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use log::trace;
use structopt::{clap::AppSettings, StructOpt};
use thiserror::Error;

use mwa_hyperdrive_srclist::{hyperdrive, read::*, rts, woden, *};

// Put various help texts in here, so that all available source list types are
// listed at compile time.
lazy_static::lazy_static! {
    static ref CONVERT_INPUT_TYPE_HELP: String =
        format!("Specifies the type of the input source list. Currently supported types: {}",
                *SOURCE_LIST_TYPES_COMMA_SEPARATED);

    static ref CONVERT_OUTPUT_TYPE_HELP: String =
        format!("Specifies the type of the output source list. May be required depending on the output filename. Currently supported types: {}",
                *SOURCE_LIST_TYPES_COMMA_SEPARATED);
}

#[derive(StructOpt, Debug)]
#[structopt(author, name = "hyperdrive srclist", about, global_settings = &[AppSettings::ColoredHelp, AppSettings::ArgRequiredElseHelp])]
enum Args {
    /// Verify that source lists can be read by hyperdrive.
    Verify {
        /// Path to the source list(s) to be verified.
        #[structopt(name = "SOURCE_LISTS", parse(from_os_str))]
        source_lists: Vec<PathBuf>,

        /// The verbosity of the program. The default is to print high-level
        /// information.
        #[structopt(short, long, parse(from_occurrences))]
        verbosity: u8,
    },

    /// Convert a source list to another supported format.
    Convert {
        #[structopt(short = "i", long, parse(from_str), help = CONVERT_INPUT_TYPE_HELP.as_str())]
        input_type: Option<String>,

        /// Path to the source list to be converted.
        #[structopt(name = "INPUT_SOURCE_LIST", parse(from_os_str))]
        input_source_list: PathBuf,

        #[structopt(short = "o", long, parse(from_str), help = CONVERT_OUTPUT_TYPE_HELP.as_str())]
        output_type: Option<String>,

        /// Path to the output source list. If the file extension is .json or
        /// .yaml, then it will written in the hyperdrive source list format. If
        /// it is .txt, then the --output-type flag should be used to specify
        /// the type of source list to be written.
        #[structopt(name = "OUTPUT_SOURCE_LIST", parse(from_os_str))]
        output_source_list: PathBuf,

        /// The verbosity of the program. The default is to print high-level
        /// information.
        #[structopt(short, long, parse(from_occurrences))]
        verbosity: u8,
    },
}

fn setup_logging(level: u8) -> Result<(), fern::InitError> {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} line {}][{}] {}",
                record.target(),
                record.line().unwrap_or(0),
                record.level(),
                message
            ))
        })
        .level(match level {
            0 => log::LevelFilter::Info,
            1 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        })
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

/// Read and print stats out for each input source list. If a source list
/// couldn't be read, print the error, and continue trying to read the other
/// source lists.
///
/// If the source list type is provided, then assume that all source lists have
/// that type.
fn verify(source_lists: Vec<PathBuf>) -> Result<(), SourceListError> {
    if source_lists.is_empty() {
        eprintln!("No source lists were supplied!");
        std::process::exit(1);
    }

    for source_list in source_lists {
        println!("{}:", source_list.display());

        let (sl, sl_type) = match read_source_list_file(&source_list, None) {
            Ok(sl) => sl,
            Err(e) => {
                println!("{}\n", e);
                continue;
            }
        };
        println!("    {}-style source list", sl_type);
        println!(
            "    {} sources, {} components\n",
            sl.len(),
            sl.iter().map(|s| s.1.components.len()).sum::<usize>()
        );
    }

    Ok(())
}

fn main() {
    // Stolen from BurntSushi. We don't return Result from main because it
    // prints the debug representation of the error. The code below prints the
    // "display" or human readable representation of the error.
    if let Err(e) = try_main() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn try_main() -> Result<(), SrclistError> {
    match Args::from_args() {
        Args::Verify {
            source_lists,
            verbosity,
        } => {
            setup_logging(verbosity).expect("Failed to initialize logging.");
            verify(source_lists)?;
        }

        Args::Convert {
            input_source_list,
            output_source_list,
            input_type,
            output_type,
            verbosity,
        } => {
            setup_logging(verbosity).expect("Failed to initialize logging.");

            let input_sl_type = match (input_type, parse_file_type(&input_source_list)) {
                // The input source list type was manually specified.
                (Some(t), _) => parse_source_list_type(&t)?,

                // Input source list type was not specified, but the file
                // extension was recognised. We can assume based on this.
                (_, Some(SourceListFileType::Yaml)) => SourceListType::Hyperdrive,
                (_, Some(SourceListFileType::Json)) => SourceListType::Hyperdrive,

                // Input source list type not specified and the file extension
                // was not recognised.
                (_, _) => SourceListType::Unspecified,
            };

            let (output_sl_type, output_file_type) =
                match (&output_type, parse_file_type(&output_source_list)) {
                    // The output source list type was manually specified.
                    (Some(t), file_type) => match (parse_source_list_type(&t)?, file_type) {
                        (SourceListType::Hyperdrive, Some(SourceListFileType::Yaml)) => {
                            (SourceListType::Hyperdrive, Some(SourceListFileType::Yaml))
                        }
                        (SourceListType::Hyperdrive, Some(SourceListFileType::Json)) => {
                            (SourceListType::Hyperdrive, Some(SourceListFileType::Json))
                        }
                        (SourceListType::Hyperdrive, _) => {
                            let ext = output_source_list
                                .extension()
                                .and_then(|os_str| os_str.to_str())
                                .map(|str| str.to_string())
                                .unwrap_or("<no extension>".to_string());
                            return Err(WriteSourceListError::InvalidHyperdriveFormat(ext).into());
                        }
                        // All other source-list types and file types are allowed.
                        (sl_type, file_type) => (sl_type, file_type),
                    },

                    // Output source list type was not specified, but the file
                    // extension was recognised. We can assume based on this.
                    (_, Some(SourceListFileType::Yaml)) => {
                        (SourceListType::Hyperdrive, Some(SourceListFileType::Yaml))
                    }
                    (_, Some(SourceListFileType::Json)) => {
                        (SourceListType::Hyperdrive, Some(SourceListFileType::Json))
                    }

                    // Not enough information is available on the output source
                    // list; we must return an error.
                    (_, _) => return Err(WriteSourceListError::NotEnoughInfo.into()),
                };

            // Read the input source list.
            let (sl, _) = mwa_hyperdrive_srclist::read::read_source_list_file(
                &input_source_list,
                Some(input_sl_type),
            )?;

            // Write the output source list.
            trace!("Attempting to write source list");
            let mut f = BufWriter::new(File::create(&output_source_list)?);

            match output_sl_type {
                SourceListType::Hyperdrive => match output_file_type {
                    Some(SourceListFileType::Yaml) => hyperdrive::source_list_to_yaml(&mut f, &sl)?,
                    Some(SourceListFileType::Json) => hyperdrive::source_list_to_json(&mut f, &sl)?,
                    // Other enum variants get handled above.
                    _ => unreachable!(),
                },

                SourceListType::Rts => rts::write_source_list(&mut f, &sl)?,

                SourceListType::Woden => woden::write_source_list(&mut f, &sl)?,

                SourceListType::AO => ao::write_source_list(&mut f, &sl)?,

                // The "unspecified" type cannot be reached from user input.
                SourceListType::Unspecified => unreachable!(),
            };

            f.flush()?;
        }
    }

    Ok(())
}

#[derive(Error, Debug)]
enum SrclistError {
    #[error("{0}")]
    SourceList(#[from] SourceListError),

    #[error("{0}")]
    WriteSourceList(#[from] WriteSourceListError),

    #[error("{0}")]
    IO(#[from] std::io::Error),
}
