use std::io::Read;
use std::path::PathBuf;
use std::result::Result;
use std::{fs::File, io::Write};

use clap::{Parser, Subcommand};
use cpio::{write_cpio, NewcBuilder};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// gets the hashes of the provided files
    GetHash {
        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
    },

    /// rename the file to its hash and create a separate file containing both the hash and the original file name
    Name {
        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
    },

    /// takes a .ncsum file and uses it to return its respective file to its original state
    Rename {
        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
    },

    /// takes a .ncsum file and uses it to check the integrity of its respective file
    Check {
        #[arg(short = 'o', long = "only-show-mismatches", default_value_t = false)]
        only_show_mismatches: bool,

        #[arg(short = 's', long = "separate-mismatches", default_value_t = false)]
        separate_mismatches: bool,

        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
    },

    Pack {
        #[arg(value_name = "FILE")]
        files: Vec<PathBuf>,
    },
}

fn get_hash(fd: &mut impl Read) -> String {
    let mut file_context = md5::Context::new();

    loop {
        let mut buffer = [0; 1024 * 1024];

        let s = match fd.read(&mut buffer) {
            Ok(s) => s,
            Err(e) => {
                println!("{e}");
                std::process::exit(1);
            }
        };

        if s == 0 {
            break;
        }

        file_context.consume(buffer);
    }

    format!("{:x}", file_context.compute())
}

trait NCSum {
    fn get_hash(&self) -> Result<String, std::io::Error>;
    fn get_suffix(&self) -> String;
}

impl NCSum for PathBuf {
    fn get_hash(&self) -> Result<String, std::io::Error> {
        let mut file = match File::open(self) {
            Ok(f) => f,
            Err(e) => {
                println!("{e:?}");
                return Result::Err(e);
            }
        };

        Result::Ok(get_hash(&mut file))
    }

    fn get_suffix(&self) -> String {
        let file_name = self
            .file_name()
            .expect("Error getting file name")
            .to_str()
            .expect("Error getting file name")
            .to_string();
        let last_dot = file_name.rfind('.').expect("Error getting file suffix");
        let ext = &file_name[last_dot..];

        String::from(ext)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct FileInfo {
    hash: String,
    old_name: String,
    new_name: String,
    ncsum_name: String,
}

impl FileInfo {
    fn new(file: &PathBuf) -> Self {
        let file_hash = match file.get_hash() {
            Ok(s) => s,
            Err(e) => {
                println!("{e}");
                std::process::exit(1);
            }
        };

        let file_suffix = file.get_suffix();

        let new_file_name = file_hash.clone() + file_suffix.as_str();
        let new_file = file
            .parent()
            .expect("Error getting file parent folder")
            .join(new_file_name);
        let ncsum_file = file
            .parent()
            .expect("Error getting file parent folder")
            .join(file_hash.clone() + ".ncsum");

        Self {
            hash: file_hash,
            old_name: String::from(file.to_str().expect("Error getting file name")),
            new_name: String::from(new_file.to_str().expect("Error getting file name")),
            ncsum_name: String::from(ncsum_file.to_str().expect("Error getting file name")),
        }
    }
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::GetHash { files } => {
            for file in files {
                let info = FileInfo::new(&file);

                println!("{}  {}", info.hash, info.old_name);
            }
        }

        Commands::Name { files } => {
            for file in files {
                let sfname = String::from(file.to_str().expect("Error getting file name"));

                if !sfname.ends_with(".ncsum") || !sfname.ends_with(".pncsum") {
                    let info = FileInfo::new(&file);

                    let mut ncsum_file = match File::create(info.ncsum_name.clone()) {
                        Ok(f) => f,
                        Err(e) => {
                            panic!("{e}: {:?}", info.ncsum_name.clone());
                        }
                    };

                    let json = match serde_json::to_string(&info) {
                        Ok(j) => j,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match ncsum_file.write_all(json.as_bytes()) {
                        Ok(n) => n,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match std::fs::rename(info.old_name.clone(), info.new_name.clone()) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    println!("{:?} -> {:?}", info.old_name, info.new_name);
                }
            }
        }

        Commands::Rename { files } => {
            for file in files {
                let sfname = String::from(file.to_str().expect("Error getting file name"));
                let mut fd: File;
                let mut info = FileInfo {
                    hash: String::new(),
                    old_name: String::new(),
                    new_name: String::new(),
                    ncsum_name: String::new(),
                };

                let mut old_name = String::new();

                if sfname.ends_with(".ncsum") {
                    fd = match File::open(sfname) {
                        Ok(f) => f,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    info = match serde_json::from_reader(fd) {
                        Ok(j) => j,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    old_name = info.old_name;
                } else if sfname.ends_with(".pncsum") {
                    fd = match File::open(sfname.clone()) {
                        Ok(f) => f,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    let mut out_fd = match File::create(sfname.replace(".pncsum", ".tncsum")) {
                        Ok(f) => f,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    loop {
                        let mut reader = match cpio::NewcReader::new(fd) {
                            Ok(f) => f,
                            Err(e) => {
                                println!("{e}");
                                std::process::exit(1);
                            }
                        };

                        if reader.entry().is_trailer() {
                            break;
                        } else if reader.entry().name().ends_with(".ncsum") {
                            info = match serde_json::from_reader(&mut reader) {
                                Ok(i) => i,
                                Err(e) => {
                                    println!("{e}");
                                    std::process::exit(1);
                                }
                            };
                        } else {
                            let mut buffer = [0; 1024 * 1024];

                            loop {
                                let len = match reader.read(&mut buffer) {
                                    Ok(i) => i,
                                    Err(e) => {
                                        println!("{e}");
                                        std::process::exit(1);
                                    }
                                };

                                if len == 0 {
                                    break;
                                }

                                match out_fd.write_all(&buffer[..len]) {
                                    Ok(_) => (),
                                    Err(e) => {
                                        println!("{e}");
                                        std::process::exit(1);
                                    }
                                };
                            }
                        }

                        fd = match reader.finish() {
                            Ok(f) => f,
                            Err(e) => {
                                println!("{e}");
                                std::process::exit(1);
                            }
                        };
                    }

                    match out_fd.flush() {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    old_name = info.old_name;
                    info.new_name = sfname.replace(".pncsum", ".tncsum");
                    info.ncsum_name = sfname;

                    let tinfo = FileInfo::new(&PathBuf::from(info.new_name.clone()));

                    if tinfo.hash != info.hash {
                        println!("An error occurred while unpacking the archive");
                        match std::fs::remove_file(tinfo.old_name) {
                            Ok(_) => (),
                            Err(e) => {
                                println!("{e}");
                                std::process::exit(1);
                            }
                        };

                        std::process::exit(1);
                    }
                }

                match std::fs::rename(info.new_name.clone(), old_name.clone()) {
                    Ok(_) => (),
                    Err(e) => {
                        println!("{e}");
                        std::process::exit(1);
                    }
                };

                match std::fs::remove_file(info.ncsum_name) {
                    Ok(_) => (),
                    Err(e) => {
                        println!("{e}");
                        std::process::exit(1);
                    }
                };

                println!("{:?} -> {:?}", info.new_name, old_name);
            }
        }

        Commands::Check {
            files,
            only_show_mismatches,
            separate_mismatches,
        } => {
            for file in files {
                let sfname = String::from(match file.to_str() {
                    Some(n) => n,
                    None => {
                        println!("error getting {:?} name", file);
                        std::process::exit(1);
                    }
                });

                if !sfname.ends_with("ncsum") {
                    continue;
                }

                let mut fd: File;
                let mut info = FileInfo {
                    hash: String::new(),
                    old_name: String::new(),
                    new_name: String::new(),
                    ncsum_name: String::new(),
                };

                let mut new_hash = String::new();

                if sfname.ends_with(".ncsum") {
                    fd = match File::open(sfname.clone()) {
                        Ok(fd) => fd,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    info = match serde_json::from_reader(fd) {
                        Ok(i) => i,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    let mut new_fd = match File::open(info.new_name.clone()) {
                        Ok(fd) => fd,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    new_hash = get_hash(&mut new_fd);
                } else if sfname.ends_with(".pncsum") {
                    fd = match File::open(sfname.clone()) {
                        Ok(fd) => fd,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    loop {
                        let mut reader = match cpio::NewcReader::new(fd) {
                            Ok(r) => r,
                            Err(e) => {
                                println!("{e}");
                                std::process::exit(1);
                            }
                        };

                        if reader.entry().is_trailer() {
                            break;
                        } else if reader.entry().name().ends_with(".ncsum") {
                            info = match serde_json::from_reader(&mut reader) {
                                Ok(i) => i,
                                Err(e) => {
                                    println!("{e}");
                                    std::process::exit(1);
                                }
                            };
                        } else {
                            new_hash = get_hash(&mut reader);
                        }

                        fd = match reader.finish() {
                            Ok(fd) => fd,
                            Err(e) => {
                                println!("{e}");
                                std::process::exit(1);
                            }
                        };
                    }
                }

                if info.hash != new_hash {
                    println!("{}: The sum does not match", info.old_name);
                }

                if !only_show_mismatches {
                    println!("{}: The sum matches", info.old_name);
                }

                if (info.hash != new_hash) && separate_mismatches {
                    let sdir = file.parent().expect("").join(info.hash);

                    match std::fs::create_dir_all(&sdir) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    }

                    let ofile = sdir.join(&info.new_name);
                    let nfile: PathBuf = sdir.join(file.file_name().unwrap().to_str().unwrap());

                    if sfname.ends_with(".ncsum") {
                        match std::fs::rename(info.new_name, ofile) {
                            Ok(_) => (),
                            Err(e) => {
                                println!("{e}");
                                std::process::exit(1);
                            }
                        }
                    }

                    match std::fs::rename(file, nfile) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }

        Commands::Pack { files } => {
            for file in files {
                let sfname = String::from(file.to_str().expect("Error getting file name"));
                let fd: File;
                let info: FileInfo;

                if sfname.ends_with(".ncsum") {
                    fd = match File::open(sfname) {
                        Ok(fd) => fd,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    info = match serde_json::from_reader(fd) {
                        Ok(i) => i,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    let pname = info.ncsum_name.replace(".ncsum", ".pncsum");
                    let mut pcontent = vec![
                        (
                            NewcBuilder::new(info.ncsum_name.clone().as_str())
                                .uid(1000)
                                .mode(0o100644),
                            match File::open(info.ncsum_name.clone()) {
                                Ok(fd) => fd,
                                Err(e) => {
                                    println!("{e}");
                                    std::process::exit(1);
                                }
                            },
                        ),
                        (
                            NewcBuilder::new(info.new_name.clone().as_str())
                                .uid(1000)
                                .mode(0o100644),
                            match File::open(info.new_name.clone()) {
                                Ok(fd) => fd,
                                Err(e) => {
                                    println!("{e}");
                                    std::process::exit(1);
                                }
                            },
                        ),
                    ];

                    let pfile = match File::create(pname.clone()) {
                        Ok(fd) => fd,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match write_cpio(pcontent.drain(..), pfile) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match std::fs::remove_file(info.ncsum_name) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match std::fs::remove_file(info.new_name) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    println!("{:?}: Created", pname);
                } else if !sfname.ends_with(".pncsum") {
                    info = FileInfo::new(&file);
                    let json = match serde_json::to_string(&info) {
                        Ok(j) => j,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    let pname = info.ncsum_name.replace(".ncsum", ".pncsum");

                    let tname = info.ncsum_name.replace(".ncsum", ".tncsum");
                    let mut tfile = match File::create(tname.clone()) {
                        Ok(fd) => fd,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match tfile.write_all(json.as_bytes()) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match tfile.flush() {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    let mut pcontent = vec![
                        (
                            NewcBuilder::new(info.ncsum_name.clone().as_str())
                                .uid(1000)
                                .mode(0o100644),
                            match File::open(tname.clone()) {
                                Ok(fd) => fd,
                                Err(e) => {
                                    println!("{e}");
                                    std::process::exit(1);
                                }
                            },
                        ),
                        (
                            NewcBuilder::new(info.new_name.clone().as_str())
                                .uid(1000)
                                .mode(0o100644),
                            match File::open(info.old_name.clone()) {
                                Ok(fd) => fd,
                                Err(e) => {
                                    println!("{e}");
                                    std::process::exit(1);
                                }
                            },
                        ),
                    ];

                    let pfile = match File::create(pname.clone()) {
                        Ok(fd) => fd,
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match write_cpio(pcontent.drain(..), pfile) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match std::fs::remove_file(info.old_name) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    match std::fs::remove_file(tname) {
                        Ok(_) => (),
                        Err(e) => {
                            println!("{e}");
                            std::process::exit(1);
                        }
                    };

                    println!("{:?}: Created", pname);
                }
            }
        }
    }
}
