use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;
use tagger::App;

#[derive(Debug, Parser)]
#[command(name = "tagger")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(arg_required_else_help = true)]
    Add {
        filename: PathBuf,
    },
    Tag {
        filename: PathBuf,
        tag: String,
    },
    Update {},
    Scan {},
    Search {
        tag: String,
    },
}

fn path_to_string(path: PathBuf) -> Result<String> {
    path.into_os_string()
        .into_string()
        .map_err(|e| anyhow!("Fucked filename"))
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let mut app = App::init("data.sqlite3")?;
    match args.command {
        Commands::Add { filename } => {
            let filename_as_str = path_to_string(filename)?;
            app.create_file(&filename_as_str)?;
        }
        Commands::Tag { filename, tag } => {
            let filename = path_to_string(filename)?;
            let file_id = match app.get_file(&filename) {
                Some(id) => id,
                None => {
                    println!("File wasn't being tracked, tracking it now...");
                    app.create_file(&filename)?
                }
            };
            let tag_id = match app.get_tag(&tag) {
                Some(id) => id,
                None => {
                    println!("Tag didn't exist, making it now...");
                    app.create_tag(&tag)?
                }
            };

            app.tag_file(tag_id, file_id);
        }
        Commands::Scan {} => {
            println!("This would walk the directory and find files to add and mark missing files as orphaned")
        }
        Commands::Update {} => {
            println!("This command would take a file, and update it's tags.")
        }

        Commands::Search { tag } => match app.get_tag(&tag) {
            None => println!("This tag doesn't exist."),
            Some(tag_id) => {
                for file in app.get_files_for_tag(tag_id)? {
                    println!("{}", file.file_name);
                }
            }
        },
    }

    Ok(())
}
