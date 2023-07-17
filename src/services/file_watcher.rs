use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;
use log::{error, info};

use sha2::{Sha256, Digest};

use crate::services::policies::memory::MemoryPolicyStore;
use crate::services::data::memory::MemoryDataStore;
use crate::services::data::DataStore;
use crate::services::policies::PolicyStore;

#[derive(Clone)]
#[derive(Debug)]
pub enum FileType {
    Data,
    Policy,
}


fn calculate_hash_file(file_path: PathBuf) -> Result<String, Box<dyn std::error::Error>> {

    if file_path.display().to_string().is_empty() {
        return Err(format!("empty path").into());
    }

    // check if file exists
    if !file_path.try_exists().unwrap_or(false) || !file_path.is_file() {
        return Err(format!("File not found: {}", file_path.display().to_string()).into());
    }

    // check if is a valid json file
    if file_path.extension().unwrap() != "json" {
        return Err(format!("Invalid file extension: {}", file_path.display().to_string()).into());
    }

    let mut file = match fs::File::open(&file_path) {
        Ok(file) => file,
        Err(err) => return Err(format!("Failed to open file: {}", err).into()),
    };

    let mut buffer = Vec::new();
    if let Err(err) = file.read_to_end(&mut buffer) {
        return Err(format!("Failed to read file: {}", err).into());
    }

    let mut hasher = Sha256::new();
    hasher.update(&buffer);

    Ok(format!("{:x}", hasher.finalize()))
}

pub async fn watch_json_file_modify(
    files: Vec<(FileType, PathBuf)>,
    data_store: Arc<MemoryDataStore>,
    policy_store: Arc<MemoryPolicyStore>,
) {

    let mut files_hash: Vec<String> = Vec::new();
    
    for (_, file) in &files {
        let hash = match calculate_hash_file(file.to_path_buf()) {
            Ok(hash) => hash,
            Err(err) => {
                error!("Failed to calculate hash file: {}", err);
                continue;
            }            
        };

        files_hash.push(hash);
    };

    if files_hash.is_empty() {
        error!("No files to watch");
        return;
    }

    let mut interval = rocket::tokio::time::interval(
        rocket::tokio::time::Duration::from_secs(5),
    );

    loop {
        interval.tick().await;

        for (index, (file_type, file)) in files.iter().enumerate() {

            if file.display().to_string().is_empty() {
                continue;
            }

            if !file.try_exists().unwrap_or(false) {
                error!("File not found: {}", file.display().to_string());
                continue;
            }

            let new_hash = calculate_hash_file(file.to_path_buf()).unwrap_or(String::new());

            if new_hash.is_empty() {
                error!("Failed to calculate {:?} hash file", file_type);
                continue;
            }

            if new_hash != files_hash[index] {
                match file_type {
                    FileType::Data => {
                        let entities = match crate::services::data::load_from_file::load_entities_from_file(PathBuf::from(file)).await {
                            Ok(entities) => entities,
                            Err(err) => {
                                error!("Failed to load entities from file: {}", err);
                                continue;
                            }
                        };
                        match data_store.update_entities(entities).await {
                            Ok(entities) => {
                                info!("Successfully updated entities from file {}: {} entities", &file.display(), entities.len());
                            }
                            Err(err) => {
                                error!("Failed to update entities: {}", err);
                                continue;
                            }
                        };
                    }
                    FileType::Policy => {
                        let policies = match crate::services::policies::load_from_file::load_policies_from_file(PathBuf::from(file)).await {
                            Ok(policies) => policies,
                            Err(err) => {
                                error!("Failed to load policies from file: {}", err);
                                continue;
                            }
                        };
                        match policy_store.update_policies(policies.into_inner()).await {
                            Ok(policies) => {
                                info!("Successfully updated policies from file {}: {} policies", &file.display(), policies.len());
                            }
                            Err(err) => {
                                error!("Failed to update policies: {}", err);
                                continue;
                            }
                        };
                    }
                }
                files_hash[index] = new_hash;
            }
        };
    }
}

pub fn init(
    data_file_path: PathBuf,
    policy_file_path: PathBuf, 
    data_store: Arc<MemoryDataStore>, 
    policy_store: Arc<MemoryPolicyStore>
) {
    if ( !data_file_path.try_exists().unwrap_or(false) && 
       !policy_file_path.try_exists().unwrap_or(false) ) ||
        ( data_file_path.display().to_string().is_empty() &&
         policy_file_path.display().to_string().is_empty() )
    {
        info!("No data or policy file found, skipping file watcher");
        return;
    }

    let mut files: Vec<(FileType, PathBuf)> = Vec::new();

    files.push((FileType::Data, data_file_path));
    files.push((FileType::Policy, policy_file_path));
    

    rocket::tokio::spawn(watch_json_file_modify(files, data_store, policy_store));
}
