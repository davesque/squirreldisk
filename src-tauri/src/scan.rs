use std::fs;
use std::ops::Not;
use std::path::Path;

use regex::{Captures, Regex};

use tauri::api::process::{Command as TauriCommand, CommandEvent};
use tauri::Manager;

use crate::MyState;

/// On macOS, cloud storage directories (iCloud, Dropbox, Google Drive, etc.)
/// cause fileproviderd to spike CPU trying to materialize file metadata.
/// This function collects paths to scan under a directory while skipping
/// known cloud-managed subdirectories nested inside Library/.
#[cfg(target_os = "macos")]
fn enumerate_skipping_cloud_dirs(dir: &Path, paths: &mut Vec<String>) {
    let cloud_dirs = ["Mobile Documents", "CloudStorage"];

    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str == "Library" && entry_path.is_dir() {
            // Enumerate Library contents, skipping cloud-managed dirs
            if let Ok(lib_entries) = fs::read_dir(&entry_path) {
                for lib_entry in lib_entries.flatten() {
                    let lib_name = lib_entry.file_name();
                    if !cloud_dirs.contains(&lib_name.to_string_lossy().as_ref()) {
                        paths.push(lib_entry.path().display().to_string());
                    }
                }
            }
        } else {
            paths.push(entry_path.display().to_string());
        }
    }
}

#[derive(Clone, serde::Serialize)]
struct Payload {
    items: u64,
    total: u64,
    errors: u64,
}

// Start scan
pub fn start(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, MyState>,
    path: String,
    ratio: String,
) -> Result<(), ()> {
    println!("Start Scanning {}", path);
    let ratio = ["--min-ratio=", ratio.as_str()].join("");

    let mut paths_to_scan: Vec<String> = Vec::new();
    paths_to_scan.push("--json-output".to_string());
    paths_to_scan.push("--progress".to_string());
    paths_to_scan.push(ratio);

    if path.eq("/") {
        let paths = fs::read_dir("/").unwrap();
        println!("{:#?}", paths);
        let banned = [
            "/dev", "/mnt", "/cdrom", "/proc", "/media", "/Volumes", "/System",
        ];

        for scan_path in paths {
            let scan_path_str = scan_path.unwrap().path();
            let path_str = scan_path_str.to_str().unwrap();
            if banned.contains(&path_str) {
                continue;
            }

            #[cfg(target_os = "macos")]
            {
                // For /Users, enumerate each home directory individually
                // to skip cloud storage dirs that cause fileproviderd to spike CPU
                if path_str == "/Users" {
                    if let Ok(users) = fs::read_dir("/Users") {
                        for user_entry in users.flatten() {
                            let user_path = user_entry.path();
                            if user_path.is_dir() {
                                enumerate_skipping_cloud_dirs(&user_path, &mut paths_to_scan);
                            }
                        }
                    }
                    continue;
                }
            }

            paths_to_scan.push(scan_path_str.display().to_string());
        }
    } else {
        paths_to_scan.push(path);
    }

    let (mut rx, child) = TauriCommand::new_sidecar("pdu")
        .expect("failed to create `my-sidecar` binary command")
        .args(paths_to_scan)
        .spawn()
        .expect("Failed to spawn sidecar");
    
    *state.0.lock().unwrap() = Some(child);

    // unlisten to the event using the `id` returned on the `listen_global` function
    // an `once_global` API is also exposed on the `App` struct

    let re = Regex::new(r"\(scanned ([0-9]*), total ([0-9]*)(?:, erred ([0-9]*))?\)").unwrap();

    tauri::async_runtime::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                CommandEvent::Stdout(line) => {
                    //println!("Stdout:{}", &line);
                    app_handle.emit_all("scan_completed", line).ok();
                }
                CommandEvent::Stderr(msg) => {
                    // println!("Stderr:{}", &msg);

                    let caps = re.captures(&msg);
                    if let Some(groups) = caps {
                        if groups.len() > 2 {
                            emit_scan_status(&app_handle, groups)
                        }
                    }
                }
                CommandEvent::Terminated(t) => {
                    println!("{t:?}");
                    // app_handle.unlisten(id);
                    // child.kill();
                }
                _ => unimplemented!(),
            };
            // if let CommandEvent::Stdout(line) = event {
            //     println!("StdErr: {}", line);
            // } else {
            //     println!("Terminated {}", event);
            // }
            // if let CommandEvent::Stderr(line) = event {
            //     println!("StdErr: {}", line);
            // }
            // if let CommandEvent::Terminated(line) = event {
            //     println!("Terminated");
            // }
        }
        Result::<(), ()>::Ok(())
    });

    Ok(())
    // thread::spawn(move || {
    //     let path = PathBuf::from(path);
    //     let mut vec: Vec<PathBuf> = Vec::new();
    //     vec.push(path);

    //     fn progress_and_error_reporter<Data>(
    //         app_handle: tauri::AppHandle,
    //     ) -> ProgressAndErrorReporter<Data, fn(ErrorReport)>
    //     where
    //         Data: Size + Into<u64> + Send + Sync,
    //         ProgressReport<Data>: Default + 'static,
    //         u64: Into<Data>,
    //     {
    //         let progress_reporter = move |report: ProgressReport<Data>| {
    //             let ProgressReport {
    //                 items,
    //                 total,
    //                 errors,
    //             } = report;
    //             let mut text = String::new();
    //             write!(
    //                 text,
    //                 "\r(scanned {items}, total {total}",
    //                 items = items,
    //                 total = total.into(),
    //             )
    //             .unwrap();
    //             if errors != 0 {
    //                 write!(text, ", erred {}", errors).unwrap();
    //             }
    //             write!(text, ")").unwrap();
    //             println!("{}", text);
    //             app_handle
    //                 .emit_all(
    //                     "scan_status",
    //                     Payload {
    //                         items: items,
    //                         total: total.into(),
    //                         errors: errors,
    //                     },
    //                 )
    //                 .unwrap();
    //         };

    //         struct TextReport<'a>(ErrorReport<'a>);

    //         impl<'a> Display for TextReport<'a> {
    //             fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
    //                 write!(
    //                     formatter,
    //                     "[error] {operation} {path:?}: {error}",
    //                     operation = self.0.operation.name(),
    //                     path = self.0.path,
    //                     error = self.0.error,
    //                 )
    //             }
    //         }

    //         let error_reporter: fn(ErrorReport) = |report| {
    //             let message = TextReport(report).to_string();
    //             println!("{}", message);
    //         };

    //         ProgressAndErrorReporter::new(
    //             progress_reporter,
    //             Duration::from_millis(100),
    //             error_reporter,
    //         )
    //     }
    //     // pub struct MyReporter {}
    //     // impl parallel_disk_usage::reporter::progress_and_error_reporter
    //     let pdu = parallel_disk_usage::app::Sub {
    //         json_output: true,
    //         direction: Direction::BottomUp,
    //         bar_alignment: BarAlignment::Right,
    //         get_data: GET_APPARENT_SIZE,
    //         files: vec,
    //         no_sort: true,
    //         min_ratio: 0.01.try_into().unwrap(),
    //         max_depth: 10.try_into().unwrap(),
    //         reporter: progress_and_error_reporter(app_handle),
    //         bytes_format: BytesFormat::MetricUnits,
    //         column_width_distribution: ColumnWidthDistribution::total(100),
    //     }
    //     .run();
    // });
}

pub fn stop(state: tauri::State<'_, MyState>) {
    state
        .0
        .lock()
        .unwrap()
        .take()
        .unwrap()
        .kill()
        .expect("State is None");
}

fn emit_scan_status(app_handle: &tauri::AppHandle, groups: Captures) {
    app_handle
        .emit_all(
            "scan_status",
            Payload {
                items: groups
                    .get(1)
                    .map_or("0", |m| m.as_str())
                    .trim_end()
                    .parse::<u64>()
                    .unwrap(),
                total: groups
                    .get(2)
                    .map_or("0", |m| m.as_str())
                    .trim_end()
                    .parse::<u64>()
                    .unwrap(),
                errors: groups
                    .get(3)
                    .map_or("0", |m| m.as_str())
                    .trim_end()
                    .parse::<u64>()
                    .unwrap(),
            },
        )
        .unwrap();
}
