#[macro_use]
extern crate clap;

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process;

use clap::{App, Arg};
use console::Term;
use rayon::prelude::*;
use serde_json::Value;
use walkdir::{DirEntry, WalkDir};

use json_structural_diff::{colorize, JsonDiff};

struct Config {
    raw: bool,
    only_keys: bool,
    color: bool,
}

fn act_on_file(
    path1: &PathBuf,
    path2: &PathBuf,
    output_path: &Option<PathBuf>,
    cfg: &Config,
) -> std::io::Result<()> {
    let buffer1 = std::fs::read(&path1).unwrap();
    let json1: Value = serde_json::from_slice(&buffer1).unwrap();
    let buffer2 = std::fs::read(path2).unwrap();
    let json2: Value = serde_json::from_slice(&buffer2).unwrap();

    if json1 != json2 {
        let json_diff = JsonDiff::diff(&json1, &json2, cfg.only_keys);
        let result = json_diff.diff.unwrap();
        let json_string = if cfg.raw {
            serde_json::to_string_pretty(&result)?
        } else {
            colorize(&result, cfg.color)
        };
        if let Some(output_path) = output_path {
            let output_filename = path1.file_name().unwrap().to_str().unwrap();
            let mut output_file = File::create(output_path.join(output_filename))?;
            writeln!(&mut output_file, "{}", json_string)?;
        } else {
            let mut term = Term::stdout();
            term.write_all(json_string.as_bytes())?;
        }
    }
    Ok(())
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

fn explore(
    path1: &PathBuf,
    path2: &PathBuf,
    output_path: &Option<PathBuf>,
    cfg: &Config,
) -> std::io::Result<()> {
    WalkDir::new(&path1)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .zip(
            WalkDir::new(&path2)
                .into_iter()
                .filter_entry(|e| !is_hidden(e)),
        )
        .par_bridge()
        .for_each(|(entry1, entry2)| {
            let entry1 = entry1.as_ref().unwrap();
            let path1_file: PathBuf = entry1.path().to_path_buf();
            let entry2 = entry2.as_ref().unwrap();
            let path2_file: PathBuf = entry2.path().to_path_buf();
            if path1_file.is_file()
                && path2_file.is_file()
                && path1_file.extension().unwrap() == "json"
                && path2_file.extension().unwrap() == "json"
            {
                act_on_file(&path1_file, &path2_file, &output_path, &cfg).unwrap();
            }
        });

    Ok(())
}

#[inline(always)]
fn exist_or_exit(path: &PathBuf, which_path: &str) {
    if !(path.exists()) {
        eprintln!(
            "The {} path `{}` is not correct",
            which_path,
            path.to_str().unwrap()
        );
        process::exit(1);
    }
}

fn main() {
    let matches = App::new("json-diff")
        .version(crate_version!())
        .author(&*env!("CARGO_PKG_AUTHORS").replace(':', "\n"))
        .about("Find the differences between two input json files")
        .arg(
            Arg::with_name("color")
                .help("Colored output")
                .short("c")
                .long("--[no-]color"),
        )
        .arg(
            Arg::with_name("raw")
                .help("Display raw JSON encoding of the diff")
                .short("j")
                .long("raw-json"),
        )
        .arg(
            Arg::with_name("keys")
                .help("Compare only the keys, ignore the differences in values")
                .short("k")
                .long("keys-only"),
        )
        .arg(
            Arg::with_name("output")
                .help("Output directory")
                .short("o")
                .long("output")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("first-json")
                .help("Old json file")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("second-json")
                .help("New json file")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let path1 = PathBuf::from(matches.value_of("first-json").unwrap());
    let path2 = PathBuf::from(matches.value_of("second-json").unwrap());

    let output_path = if let Some(path) = matches.value_of("output") {
        let path = PathBuf::from(path);
        exist_or_exit(&path, "output");
        Some(path)
    } else {
        None
    };

    exist_or_exit(&path1, "first");
    exist_or_exit(&path2, "second");

    let color = if output_path.is_none() {
        matches.is_present("color")
    } else {
        false
    };
    let raw = matches.is_present("raw");
    let only_keys = matches.is_present("keys");

    let cfg = Config {
        raw,
        only_keys,
        color,
    };

    if path1.is_dir() && path2.is_dir() {
        explore(&path1, &path2, &output_path, &cfg).unwrap();
    } else if (path1.is_dir() && !path2.is_dir()) || (!path1.is_dir() && path2.is_dir()) {
        eprintln!("Both paths should be a directory or a file",);
        process::exit(1);
    } else {
        act_on_file(&path1, &path2, &output_path, &cfg).unwrap();
    }
}
