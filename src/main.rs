use clap::Parser;
use nom_exif::{ExifIter, MediaParser, MediaSource, TrackInfo};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

#[derive(Debug, Parser)]
struct Cli {
    #[structopt(short = 'i')]
    input_folder: String,
    #[structopt(short = 'o')]
    output_folder: String,
    #[structopt(short = 'd')]
    dry_run: bool,
}

struct Logger {
    is_debug: bool,
}

impl Logger {
    fn new() -> Self {
        let is_debug =
            env::var("DEBUG").map_or(false, |env_value| env_value.to_lowercase() == "true");
        println!("Debug mode: {is_debug}");

        Self { is_debug }
    }

    fn info(&self, message: String) {
        println!("ℹ️ INFO: {}", message);
    }

    fn warning(&self, message: String) {
        println!("⛔️ WARNING: {}", message);
    }

    fn debug(&self, message: String) {
        if self.is_debug {
            println!("🪲 DEBUG: {}", message);
        }
    }

    fn error(&self, message: String) {
        eprintln!("💣 ERROR: {}.", message);
    }
}

fn main() {
    let logger = Logger::new();
    let args = Cli::parse();

    logger.debug("Reading input folder...📖".to_owned());

    let folder = fs::read_dir(&args.input_folder).unwrap_or_else(|error| {
        logger.error(format!(
            "Error while reading input directory. Details: {error}"
        ));

        process::exit(1);
    });

    logger.info("Iterating through files in the folder...🏃".to_owned());

    let folder_collection: Vec<_> = folder.collect();
    let amount_of_files = folder_collection.len();

    for (index, file) in folder_collection.into_iter().enumerate() {
        let path = file.unwrap().path();
        let file_name = path.file_name().unwrap().to_str().unwrap();

        logger.info(format!(
            "Processing file {index} of {amount_of_files} ⏳",
            index = index + 1
        ));
        logger.debug(format!("File name: {file_name} 🪲"));

        let extension = path.extension().unwrap().to_str().unwrap().to_lowercase();

        if !["jpg", "jpeg", "png", "mov", "mp4"].contains(&extension.as_str()) {
            continue;
        }

        let destination_subfolder_name =
            match determine_subfolder_name_from_metadata(&logger, path.clone()) {
                Some(value) => value,
                None => continue,
            };

        let new_file_path_str = format!(
            "{output_folder}/{destination_subfolder_name}/{file_name}",
            output_folder = args.output_folder,
            destination_subfolder_name = destination_subfolder_name,
            file_name = file_name,
        );
        let new_file_path = Path::new(&new_file_path_str);

        if args.dry_run {
            logger.info("Dry run... (Skipping moving the file.)".to_owned());
            logger.info(format!(
                "Would move {file_name} to {new_file_path}",
                file_name = file_name,
                new_file_path = new_file_path_str
            ));
        } else {
            fs::create_dir_all(new_file_path.parent().unwrap()).unwrap();
            fs::rename(path.clone(), new_file_path).unwrap();
        }
    }
}

fn determine_subfolder_name_from_metadata(
    logger: &Logger,
    path: std::path::PathBuf,
) -> Option<String> {
    let mut destination_subfolder_name = "NOT_QUALIFIED".to_owned();
    let mut parser = MediaParser::new();
    let media_source = match MediaSource::file_path(path) {
        Ok(source) => source,
        Err(error) => {
            logger.warning(format!(
                "Couldn't get metadata of the file so skipping it. Details: {error}"
            ));
            return None;
        }
    };

    if media_source.has_exif() {
        let mut iter: ExifIter = match parser.parse(media_source) {
            Ok(iter) => iter,
            Err(error) => {
                logger.warning(format!("Failed parsing Exif data. Details: {error}"));

                return Some(destination_subfolder_name);
            }
        };

        let exif_entry = iter.find(|entry| {
            matches!(
                entry.tag(),
                Some(nom_exif::ExifTag::DateTimeOriginal | nom_exif::ExifTag::CreateDate)
            )
        });
        let exif_entry = match exif_entry {
            Some(entry) => entry,
            None => {
                logger.warning(
                    "Failed reading Exif data. Details: No DateTimeOriginal or CreateDate tag found.".to_owned(),
                );

                return Some(destination_subfolder_name);
            }
        };
        let exif_data = exif_entry.get_value().unwrap().as_time();

        destination_subfolder_name = exif_data.unwrap().date_naive().format("%Y").to_string();
    } else if media_source.has_track() {
        let info: TrackInfo = match parser.parse(media_source) {
            Ok(info) => info,
            Err(error) => {
                logger.warning(format!("Failed parsing track data. Details: {error}"));

                return Some(destination_subfolder_name);
            }
        };
        let track_data = info.get(nom_exif::TrackInfoTag::CreateDate).unwrap();

        destination_subfolder_name = track_data
            .as_time()
            .unwrap()
            .date_naive()
            .format("%Y")
            .to_string();
    } else {
        logger.warning("No Exif or Track data found so skipping the current file.".to_owned());
    }

    Some(destination_subfolder_name)
}
