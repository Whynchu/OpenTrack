use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

use workstation_core::cpr::{
    CprStructure, decode_root_label_strings, printable_ascii_runs,
    printable_ascii_runs_with_offsets,
};

const MAX_ASCII_RUN_OUTPUT: usize = 10_000;

fn main() -> ExitCode {
    let args = env::args_os().skip(1);
    let mut all_ascii_runs = false;
    let mut show_blocks = false;
    let mut show_root_labels = false;
    let mut path = None;
    for arg in args {
        if arg == "--all-ascii-runs" {
            all_ascii_runs = true;
        } else if arg == "--blocks" {
            show_blocks = true;
        } else if arg == "--root-labels" {
            show_root_labels = true;
        } else if path.is_none() {
            path = Some(arg);
        } else {
            print_usage();
            return ExitCode::from(2);
        }
    }
    let Some(path) = path else {
        print_usage();
        return ExitCode::from(2);
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read project: {error}");
            return ExitCode::FAILURE;
        }
    };

    let structure = match CprStructure::parse(&bytes) {
        Ok(structure) => structure,
        Err(error) => {
            eprintln!("{}: {error}", path.to_string_lossy());
            return ExitCode::FAILURE;
        }
    };
    let header = structure.header;

    println!("file: {}", path.to_string_lossy());
    println!("bytes: {}", bytes.len());
    println!(
        "root: {}/{}",
        String::from_utf8_lossy(&header.container_id),
        String::from_utf8_lossy(&header.root_id)
    );
    println!("declared big-endian length: {}", header.declared_length);
    println!(
        "length matches observed template rule: {}",
        header.length_matches_file
    );

    if show_blocks {
        println!("outer blocks: {}", structure.blocks.len());
        for (index, block) in structure.blocks.iter().enumerate() {
            println!(
                "block[{index}] tag={} header=0x{:08X} payload=0x{:08X} length={} end=0x{:08X}",
                String::from_utf8_lossy(&block.tag),
                block.header_offset,
                block.payload_offset,
                block.payload_length,
                block.end_offset
            );
        }
    }

    if show_root_labels {
        for block in structure
            .blocks
            .iter()
            .filter(|block| block.tag == *b"ROOT")
        {
            let payload = &bytes[block.payload_offset..block.end_offset];
            match decode_root_label_strings(payload) {
                Some(labels) => println!(
                    "root-labels[0x{:08X}]: {}",
                    block.header_offset,
                    labels.join(" / ")
                ),
                None => println!(
                    "root-labels[0x{:08X}]: <not a printable length-prefixed string sequence>",
                    block.header_offset
                ),
            }
        }
    }

    if all_ascii_runs {
        let runs = printable_ascii_runs_with_offsets(&bytes, 5);
        println!("printable ASCII runs: {}", runs.len());
        for run in runs.iter().take(MAX_ASCII_RUN_OUTPUT) {
            println!(
                "ascii-run[0x{:08X}, {}]: {}",
                run.offset,
                run.text.len(),
                run.text
            );
        }
        if runs.len() > MAX_ASCII_RUN_OUTPUT {
            println!(
                "... {} further ASCII runs omitted",
                runs.len() - MAX_ASCII_RUN_OUTPUT
            );
        }
    } else if !show_blocks && !show_root_labels {
        let mut seen = HashSet::new();
        let mut printed = 0;
        for text in printable_ascii_runs(&bytes, 5) {
            if seen.insert(text.clone()) {
                println!("text: {text}");
                printed += 1;
                if printed == 40 {
                    println!("... further printable strings omitted");
                    break;
                }
            }
        }
    }

    ExitCode::SUCCESS
}

fn print_usage() {
    eprintln!("usage: cpr-inspect [--blocks] [--root-labels] [--all-ascii-runs] <project.cpr>");
}
