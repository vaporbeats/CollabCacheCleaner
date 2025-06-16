use tauri::Manager;
use tauri::{AppHandle, State};
use tauri::async_runtime::Mutex;
use tauri_plugin_opener::OpenerExt;
use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;
use walkdir::WalkDir;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
struct ProjectDef {
      id: String,
    name: String,
    year: u16,
    days: u64,
    size: u64,
}

pub struct ProjectCache(pub Mutex<HashMap<String, PathBuf>>);

pub struct AppPaths {
    revit_cc_base: PathBuf,
}

const MINIMUM_VERSION: u16 = 2018;
const MAXIMUM_VERSION: u16 = 2038;

#[tauri::command]
async fn get_projects(
    paths: State<'_, AppPaths>,
    cache: State<'_, ProjectCache>
) -> Result<Vec<ProjectDef>, ()> { // Result is eiter a Vector on Ok, or a void error.

    // Container vector
    let mut all_projects = Vec::new();

    // Clear cache on run
    cache.0.lock().await.clear();

    // Iterate over each year
    for vers in MINIMUM_VERSION..=MAXIMUM_VERSION {

        // Create path: %localappdata%/Autodesk/Revit/Autodesk Revit {vers}/CollaborationCache
        let vers_path = &paths.revit_cc_base
            .join(format!("Autodesk Revit {}", vers))
            .join("CollaborationCache");

        // Check that the path exists first
        if !vers_path.exists() {
            continue;
        }

        // Get an iterator of user folders in the current version folder
        let user_folders = match fs::read_dir(vers_path) {
            Ok(folders) => folders,
            Err(_) => continue,
        };

        // Loop over the user folders
        for user_folder in user_folders {

            // Check that we've gotten a user folder on this iteration
            let user_folder_entry = match user_folder {
                Ok(entry) => entry,
                Err(_) => continue,
            };

            // Check that it's a directory
            let user_folder_path = user_folder_entry.path();
            if !user_folder_path.is_dir() {
                continue;
            }

            // Get an iterator for the project folders in the current user folder
            let project_folders = match fs::read_dir(user_folder_path) {
                Ok(folders) => folders,
                Err(_) => continue,
            };

            // Loop over the project folders
            for project_folder in project_folders {

                // Check that we've gotten a project folder this iteration
                let project_folder_entry = match project_folder {
                    Ok(entry) => entry,
                    Err(_) => continue,
                };

                // Get the path to the folder and make an id for it
                let project_folder_path = project_folder_entry.path();
                let path_id = project_folder_path.to_string_lossy().into_owned();

                // Get the name of the folder itself to show in the UI
                let project_name = project_folder_entry.file_name().to_string_lossy().into_owned();

                // Set up container variables for the size and creation date of the newest file
                let mut total_size: u64 = 0;
                let mut youngest_file_time: Option<std::time::SystemTime> = None;

                // Walk the directory for each file/folder
                for entry in WalkDir::new(&project_folder_path) {
                    // Check that we're able to get the entry from the current step of the walk
                    if let Ok(entry) = entry {
                        // Make sure that entry returns metadata
                        if let Ok(metadata) = entry.metadata() {
                            // Check the current entry is a file (i.e. not a folder)
                            if !metadata.is_file() {
                                continue;
                            }

                            // Add the size of the current entry to the total size for the project
                            total_size += metadata.len();

                            // Check that the entry has a modified time
                            if let Ok(modified_time) = metadata.modified() {
                                // Check if the new time is younger than the current youngest time
                                match youngest_file_time {
                                    Some(current_youngest) if modified_time > current_youngest => {
                                        youngest_file_time = Some(modified_time);
                                    }
                                    None => {
                                        youngest_file_time = Some(modified_time);
                                    }
                                    // Do nothing if its not
                                    _ => {}
                                }
                            }
                        }    
                    }
                }

                // Convert the system time into a number of days
                let mut days_old: u64 = 0;
                if let Some(youngest_time) = youngest_file_time {
                    if let Ok(duration) = std::time::SystemTime::now().duration_since(youngest_time) {
                        days_old = duration.as_secs() / 86400; // 86400 = 60*60*24
                    }
                }

                // Set up the definition for the project to pass to the frontend
                let new_project = ProjectDef {
                    id: path_id.clone(),
                    name: project_name,
                    year: vers,
                    size: total_size,
                    days: days_old,
                };

                // Push that definition onto the output vector
                all_projects.push(new_project);

                // Add the id:path pair to the cach to keep a referenceable pristine PathBuf copy of the path on the backend
                cache.0.lock().await.insert(path_id, project_folder_path);
            }
        }
    }
    // return all_projects on the Ok of the Result<>
    Ok(all_projects)
}

#[tauri::command]
async fn open_project(
    id: String,
    cache: State<'_, ProjectCache>,
    app: AppHandle
) -> Result<(), String> { // Does not return an Ok, only returns an Err if there is an issue
    // Try and pull the path from the id on the cache
    if let Some(path) = cache.0.lock().await.get(&id) {
        // Start the app.opener
        app.opener()
            // Open the path, passing "none" as the handler to use the default explorer window
            .open_path(path.to_string_lossy(), None::<&str>)
            // Map the error to e and return it
            .map_err(|e| e.to_string())
    } else {
        // If we didn't get anything looking for the id in the cache, log an error.
        Err("Project not found in cache".to_string())
    }
}

#[tauri::command]
fn open_vers(
    vers: u16, // We expect a number like "2025", this should be a "year" from ProjectDef
    paths: State<'_, AppPaths>,
    app: AppHandle
) -> Result<(), String> {
    // Create path: %localappdata%/Autodesk/Revit/Autodesk Revit {vers}/CollaborationCache
    let vers_path = &paths.revit_cc_base
            .join(format!("Autodesk Revit {}", vers))
            .join("CollaborationCache");
    // Start the app.opener
    app.opener()
        // Open the path, passing "none" as the handler to use the default explorer window
        .open_path(vers_path.to_string_lossy(), None::<&str>)
        // Map the error to e and return it
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_folder(
    id: String,
    cache: State<'_, ProjectCache>
) -> Result<(), String> { // Does not return an Ok, only retuns an Err if there is an issue
    // Try and pull the path from the id on the cache
    if let Some(path_to_delete) = cache.0.lock().await.remove(&id) {
        // If we get a pth, try and remove it
        match fs::remove_dir_all(&path_to_delete) {
            Ok(_) => { // if remove_dir_all returns an Ok
                println!("Successfully Deleted Directory: {:?}", path_to_delete);
                Ok(())
            },
            Err(e) => { // if remove_dir_all returns an error, we should pass it along
                let error_message = format!("Failed to delete directory {:?}: {}", path_to_delete, e);
                println!("{}", error_message);
                Err(error_message)
            }
        }
    } else {
        // If we didn't get anything looking for the id in the cache, log an error.
        Err(format!("Project with ID '{}' not found in cache.", id))
    }
}


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let handle = app.handle();
            let path_resolver = handle.path();

            let local_data = path_resolver
                .local_data_dir()
                .expect("Failed to get local data directory.");

            let revit_cc_base = local_data
                .join("Autodesk")
                .join("Revit");

            app.manage(AppPaths { revit_cc_base });
            app.manage(ProjectCache(Mutex::new(HashMap::new())));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![get_projects, open_project, open_vers, delete_folder])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
