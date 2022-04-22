extern crate serde;

use std::format;
use which::which_all;
use io::prelude::*;
use std::fs;
use std::io::{self};
use std::process::Command;
use std::io::{BufReader};

use std::error::Error;
use std::path::{Component, Path, PathBuf};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize, Serialize, Debug)]
enum FileType {
  Source,
  Object,
  Library,
  Executable,
}

#[derive(Deserialize, Serialize, Debug)]
struct File {
  id: String,
  r#type: FileType,
  name: String,
}

#[derive(Deserialize, Serialize, Debug)]
enum StageType {
  Compilation,
  Link,
  Archiving,
}

#[derive(Deserialize, Serialize, Debug)]
struct Stage {
  id: String,
  inputs: Vec<String>,
  outputs: Vec<String>,
  r#type: StageType,
  duration: u128,
}

#[derive(Deserialize, Serialize, Debug)]
struct Report {
  files: Vec<File>,
  stages: Vec<Stage>,
}

// Return a PathBuf if file exists or an error.
fn file_exists(path: &str) -> Result<PathBuf, Box<dyn Error>> {
    let path_buf = PathBuf::from(path);
    if path_buf.exists() {
        Ok(path_buf)
    } else {
        Err(format!("{} does not exists", path).into())
    }
}

fn get_unique_id() -> String {
  let mut rnd: fs::File = fs::File::open("/dev/urandom").unwrap();
  let mut buffer = [0; 8];
  let _ = rnd.read(&mut buffer[..]).unwrap();
  let hexmap = vec!['0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'];
  let id = format!("{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}{}",
    hexmap[buffer[0] as usize & 0x0F],
    hexmap[buffer[0] as usize >> 4 & 0x0F],
    hexmap[buffer[1] as usize & 0x0F],
    hexmap[buffer[1] as usize >> 4 & 0x0F],
    hexmap[buffer[2] as usize & 0x0F],
    hexmap[buffer[2] as usize >> 4 & 0x0F],
    hexmap[buffer[3] as usize & 0x0F],
    hexmap[buffer[3] as usize >> 4 & 0x0F],
    hexmap[buffer[4] as usize & 0x0F],
    hexmap[buffer[4] as usize >> 4 & 0x0F],
    hexmap[buffer[5] as usize & 0x0F],
    hexmap[buffer[5] as usize >> 4 & 0x0F],
    hexmap[buffer[6] as usize & 0x0F],
    hexmap[buffer[6] as usize >> 4 & 0x0F],
    hexmap[buffer[7] as usize & 0x0F],
    hexmap[buffer[7] as usize >> 4 & 0x0F],
  );
  return id;
}

// Returns the epoch in microseconds
fn date_now() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
      .duration_since(UNIX_EPOCH).unwrap();
    return since_the_epoch.as_micros();
}

fn write_timestamp(filepath: &str) {
  let mut file = std::fs::OpenOptions::new()
    .create(true)
    .write(true)
    .open(filepath).unwrap();
  let now = date_now() / 1000;
  write!(file, "{}", now).unwrap();
}

fn check_timestamp(timestamp_filepath: &str) -> bool {
  let timestamp_file = PathBuf::from(timestamp_filepath);
  let timestamp = fs::read_to_string(timestamp_file).unwrap().parse::<u128>().unwrap();
  let now = date_now() / 1000;
  write_timestamp(timestamp_filepath);
  return now - timestamp > 60000 * 5;
}

// Periodically warns the use that they are using eyec.
fn should_warn() -> bool {
  let timestamp_filepath = "/tmp/eyec.timestamp";
  match file_exists(timestamp_filepath) {
    Ok(_) => check_timestamp(timestamp_filepath),
    Err(_) => {
      write_timestamp(timestamp_filepath);
      return true;
    }
  }
}

// Will go through the PATH to retrieve the actual program we need to run.
fn get_real_program_path(path: &str) -> String {
  let provided_path = Path::new(path);
  let program = if let Component::Normal(program) = provided_path.components().last().unwrap() {
    program.to_str().unwrap()
  } else {
    unreachable!();
  };

  let current_exe = std::env::current_exe().unwrap();
  // get the first one that is not a symlink
  return which_all(program).unwrap()
    // converts all the symlinks to their pointed path in canonical form
    .map(|p| p.canonicalize().unwrap())
    // find the first path that is not eyec
    .find(|p| p.to_string_lossy() != current_exe.to_string_lossy())
    .expect(&format!("Couldn't no find {} in the PATH", program))
    .to_string_lossy().to_string();
}

fn analyze(program: &String, args: &Vec<String>, report: &mut Report, duration: u128) {
  if ["cc", "c++", "gcc", "g++"].iter().any(|compiler| program.contains(compiler)) {
    // Look for -c (object compilation)
    if let Some(_compile_flag_position) = args.iter().position(|a| a == "-c") {
      // Gather the source files
      let sources = args.iter()
        .filter(|a| a.ends_with(".c") || a.ends_with(".cc") || a.ends_with(".cpp")
          || a.ends_with(".cxx") || a.ends_with(".cx") || a.ends_with(".c++"))
        .collect::<Vec<_>>();
      // If only one source file, then look for output option -o
      let object = if sources.len() == 1 {
        if let Some(output_flag_position) = args.iter().position(|a| a == "-o") {
          args.iter().nth(output_flag_position + 1)
        } else { None }
      } else { None };
      // Create the input/output data structure for the report
      let inputs = sources.iter().map(|source| File {
        id: get_unique_id(),
        r#type: FileType::Source,
        name: source.to_string(),
      }).collect::<Vec<_>>();
      let mut outputs = vec![];
      if let Some(object) = object {
        outputs.push(File {
          id: get_unique_id(),
          r#type: FileType::Object,
          name: object.to_string(),
        });
      }
      // Fill up report
      let stage = Stage {
        id: get_unique_id(),
        inputs: inputs.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
        outputs: outputs.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
        r#type: StageType::Compilation,
        duration,
      };
      report.stages.push(stage);
      report.files.extend(inputs);
      report.files.extend(outputs);
    } else {
      // Are we outputing something without a -c, maybe an executable
      if let Some(output_flag_position) = args.iter().position(|a| a == "-o") {
        // Retrieve the executable
        let executable = File {
          id: get_unique_id(),
          r#type: FileType::Executable,
          name: args.iter().nth(output_flag_position + 1).unwrap().to_string(),
        };
        // Retrieve the implicit libraries
        let implicit_libraries = args.iter()
          .filter(|a|
            a.starts_with("-l") && a.len() > 2 && a.chars().nth(2).unwrap().is_alphanumeric()
          )
          .map(|a| format!("lib{}.a", &a[2..]))
          .map(|filename| File {
            id: get_unique_id(),
            r#type: FileType::Library,
            name: filename,
          })
          .collect::<Vec<_>>();
        // Retrieve the explicit libraries
        let explicit_libraries = args.iter()
          .filter(|a| a.ends_with(".a"))
          .map(|filename| File {
            id: get_unique_id(),
            r#type: FileType::Library,
            name: filename.to_string(),
          })
          .collect::<Vec<_>>();
        // Retrieve the object files
        let objects = args.iter()
          .filter(|a| a.ends_with(".o"))
          .map(|filename| File {
            id: get_unique_id(),
            r#type: FileType::Object,
            name: filename.to_string(),
          })
          .collect::<Vec<_>>();
        // Fill up report
        report.stages.push(Stage {
          id: get_unique_id(),
          inputs: implicit_libraries.iter()
            .chain(explicit_libraries.iter())
            .chain(objects.iter())
            .map(|f| f.id.clone()).collect::<Vec<_>>(),
          outputs: vec![executable.id.clone()],
          r#type: StageType::Link,
          duration,
        });
        report.files.push(executable);
        report.files.extend(implicit_libraries);
        report.files.extend(explicit_libraries);
        report.files.extend(objects);
      }
    }
  } else if program.contains("ar") {
    // Retrieve the libraries
    let libraries = args.iter()
      .filter(|a| a.ends_with(".a"))
      .map(|filename| File {
        id: get_unique_id(),
        r#type: FileType::Library,
        name: filename.to_string(),
      })
      .collect::<Vec<_>>();
    // Retrieve the object files
    let objects = args.iter()
      .filter(|a| a.ends_with(".o"))
      .map(|filename| File {
        id: get_unique_id(),
        r#type: FileType::Object,
        name: filename.to_string(),
      })
      .collect::<Vec<_>>();
    // Fill up report
    report.stages.push(Stage {
      id: get_unique_id(),
      inputs: objects.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
      outputs: libraries.iter().map(|f| f.id.clone()).collect::<Vec<_>>(),
      r#type: StageType::Archiving,
      duration,
    });
    report.files.extend(libraries);
    report.files.extend(objects);
  }
}

fn load_report(report_filename: &str) -> Report {
  let report_file = std::fs::OpenOptions::new()
    .read(true)
    .open(report_filename);
  if let Ok(report_file) = report_file {
    let reader = BufReader::new(report_file);
    if let Ok(report) = serde_json::from_reader(reader) {
      return report;
    }
  }
  Report { files: vec![], stages: vec![] }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  if should_warn() { println!("warning: your compiler executable is being wrapped by eyec."); }

  let report_filename = &std::env::var("EYEC_REPORT").unwrap_or(
    format!("{}/eyec-report.json", std::env::current_dir()?.to_string_lossy()));
  let mut report = load_report(report_filename);

  let then = date_now();
  let args: Vec<_> = std::env::args().collect();
  let real_program_path = &get_real_program_path(&args[0]);
  println!("{:?} -> {}", args, real_program_path);
  // Execute the actual program
  let mut child = Command::new(real_program_path)
    .args(&args.iter().skip(1).collect::<Vec<_>>())
    .spawn()
    .expect(&format!("{} failed to start", real_program_path));

  let _ = child.wait()
    .expect("failed to wait on g++");
  // Enrich the report with what we just observed
  analyze(real_program_path, &args, &mut report, date_now() - then);

  let report_file = std::fs::OpenOptions::new()
    .write(true)
    .create(true)
    .truncate(true)
    .open(report_filename).expect(&format!("could not open {} for writing", report_filename));
  // println!("report {:?}", report);
  serde_json::to_writer(&report_file, &report)?;

  Ok(())
}
