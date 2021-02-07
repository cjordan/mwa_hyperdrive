// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

/*!
Handle source list files.

The executable generated by this file can verify that hyperdrive can read source
files, as well as convert between supported source list formats.
 */

use std::fs::File;
use std::path::PathBuf;

use anyhow::bail;
use log::{debug, warn};
use structopt::{clap::AppSettings, StructOpt};

use mwa_hyperdrive_core::SourceList;
use mwa_hyperdrive_srclist::{hyperdrive, read::*, rts, woden, *};

// Put various help texts in here, so that all available source list types are
// listed at compile time.
lazy_static::lazy_static! {
    static ref VERIFY_INPUT_TYPE_HELP: String =
        format!("The type of source lists being verified. This is only really useful if they are .txt files, because it's ambiguous if these are RTS or WODEN source lists. Currently supported types: {}",
                *SOURCE_LIST_TYPES_COMMA_SEPARATED);

    static ref CONVERT_INPUT_TYPE_HELP: String =
        format!("If the input source list is a .txt file, this flag specifies the type of source list read. Currently supported types: {}",
                *SOURCE_LIST_TYPES_COMMA_SEPARATED);

    static ref CONVERT_OUTPUT_TYPE_HELP: String =
        format!("If the output source list is a .txt file, then this flag specifies the type of source list written. Currently supported types: {}",
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

        #[structopt(short = "i", long, parse(from_str), help = VERIFY_INPUT_TYPE_HELP.as_str())]
        input_type: Option<String>,

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

fn source_list_type_compatible_with_file_type(
    sl_type: &SourceListType,
    file_type: &SourceListFileType,
) -> Result<(), anyhow::Error> {
    let exit = |slt, ft| {
        bail!(
            "Source list type {:?} cannot be read from or written to a file type {:?}",
            slt,
            ft
        )
    };

    match sl_type {
        SourceListType::Hyperdrive => match file_type {
            SourceListFileType::Json | SourceListFileType::Yaml => Ok(()),
            _ => exit(sl_type, file_type),
        },
        SourceListType::Rts | SourceListType::Woden | SourceListType::AO => match file_type {
            SourceListFileType::Txt => Ok(()),
            _ => exit(sl_type, file_type),
        },
    }
}

/// Read and print stats out for each input source list. If a source list
/// couldn't be read, print the error, and continue trying to read the other
/// source lists.
///
/// If the source list type is provided, then assume that all source lists have
/// that type.
fn verify(
    source_lists: Vec<PathBuf>,
    source_list_type: Option<SourceListType>,
) -> Result<(), anyhow::Error> {
    if source_lists.is_empty() {
        bail!("No source lists were supplied!");
    }

    for source_list in source_lists {
        println!("{}:", source_list.display());

        let file_type = parse_file_type(&source_list)?;
        let sl_type = match &source_list_type {
            Some(slt) => slt.clone(),
            // If the source list type wasn't provided, then try to guess from
            // the file type.
            None => match &file_type {
                SourceListFileType::Json | SourceListFileType::Yaml => SourceListType::Hyperdrive,
                SourceListFileType::Txt => {
                    warn!("Assuming that the input source list is RTS style");
                    SourceListType::Rts
                }
            },
        };

        // Check that the source list type is compatible with the file type.
        match source_list_type_compatible_with_file_type(&sl_type, &file_type) {
            Ok(()) => (),
            Err(e) => println!("{}\n", e),
        }

        let sl: SourceList = {
            let mut f = std::io::BufReader::new(File::open(&source_list)?);

            match sl_type {
                // The following could probably be cleaned up with macros, but
                // I'm not comfortable crossing that bridge yet...
                SourceListType::Hyperdrive => match file_type {
                    SourceListFileType::Json => match hyperdrive::source_list_from_json(&mut f) {
                        Ok(sl) => sl,
                        Err(e) => {
                            println!("{}\n", e);
                            continue;
                        }
                    },
                    SourceListFileType::Yaml => match hyperdrive::source_list_from_yaml(&mut f) {
                        Ok(sl) => sl,
                        Err(e) => {
                            println!("{}\n", e);
                            continue;
                        }
                    },
                    // Other enum variants get handled above.
                    _ => unreachable!(),
                },

                SourceListType::Rts => match rts::parse_source_list(&mut f) {
                    Ok(sl) => sl,
                    Err(e) => {
                        println!("{}\n", e);
                        continue;
                    }
                },

                SourceListType::Woden => match woden::parse_source_list(&mut f) {
                    Ok(sl) => sl,
                    Err(e) => {
                        println!("{}\n", e);
                        continue;
                    }
                },

                SourceListType::AO => match ao::parse_source_list(&mut f) {
                    Ok(sl) => sl,
                    Err(e) => {
                        println!("{}\n", e);
                        continue;
                    }
                },
            }
        };

        println!(
            "{} sources, {} components\n",
            sl.len(),
            sl.iter().map(|s| s.1.components.len()).sum::<usize>()
        );
    }

    Ok(())
}

fn main() -> Result<(), anyhow::Error> {
    match Args::from_args() {
        Args::Verify {
            source_lists,
            input_type,
            verbosity,
        } => {
            setup_logging(verbosity).expect("Failed to initialize logging.");

            let input_sl_type = input_type.map(|t| parse_source_list_type(&t).unwrap());
            verify(source_lists, input_sl_type)
        }

        Args::Convert {
            input_source_list,
            output_source_list,
            input_type,
            output_type,
            verbosity,
        } => {
            setup_logging(verbosity).expect("Failed to initialize logging.");

            let input_file_type = parse_file_type(&input_source_list)?;
            let output_file_type = parse_file_type(&output_source_list)?;

            let input_sl_type = match input_type {
                // The input source list type was manually specified.
                Some(t) => parse_source_list_type(&t)?,

                // Input source list type not specified; try to get the type
                // from the output file's extension.
                None => match &input_file_type {
                    SourceListFileType::Json | SourceListFileType::Yaml => {
                        SourceListType::Hyperdrive
                    }
                    SourceListFileType::Txt => {
                        warn!("Assuming that the input source list is RTS style");
                        SourceListType::Rts
                    }
                },
            };
            let output_sl_type = match &output_type {
                // The output source list type was manually specified.
                Some(t) => parse_source_list_type(&t)?,

                // Output source list type not specified; try to get the type
                // from the output file's extension.
                None => match &output_file_type {
                    SourceListFileType::Json | SourceListFileType::Yaml => {
                        SourceListType::Hyperdrive
                    }
                    SourceListFileType::Txt => {
                        warn!("Assuming that the output source list is RTS style");
                        SourceListType::Rts
                    }
                },
            };

            // Check that the source list types are compatible with
            // corresponding file types.
            source_list_type_compatible_with_file_type(&input_sl_type, &input_file_type)?;
            source_list_type_compatible_with_file_type(&output_sl_type, &output_file_type)?;

            // Read the input source list.
            let sl = mwa_hyperdrive_srclist::read::read_source_list_file(
                &input_source_list,
                &input_sl_type,
            )?;

            // Write the output source list.
            debug!("Attempting to write source list");
            let mut f = std::io::BufWriter::new(File::create(&output_source_list)?);

            match output_sl_type {
                SourceListType::Hyperdrive => match output_file_type {
                    SourceListFileType::Json => hyperdrive::source_list_to_json(&mut f, &sl)?,
                    SourceListFileType::Yaml => hyperdrive::source_list_to_yaml(&mut f, &sl)?,
                    // Other enum variants get handled above.
                    _ => unreachable!(),
                },

                SourceListType::Rts => rts::write_source_list(&mut f, &sl)?,

                SourceListType::Woden => woden::write_source_list(&mut f, &sl)?,

                SourceListType::AO => ao::write_source_list(&mut f, &sl)?,
            };

            Ok(())
        }
    }
}
