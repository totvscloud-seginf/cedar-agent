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
pub enum FileType {
    Data,
    Policy,
}


fn calculate_hash_file(file_path: &str) -> Result<String, Box<dyn std::error::Error>> {
    let path = PathBuf::from(file_path);

    // check if file exists
    if !path.exists() {
        return Ok("".to_string());
    }

    // check if is a valid json file
    if path.extension().unwrap() != "json" {
        return Ok("".to_string());
    }

    let mut file = match fs::File::open(&path) {
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
    files: Vec<(FileType, String)>,
    data_store: Arc<MemoryDataStore>,
    policy_store: Arc<MemoryPolicyStore>,
) {

    let mut files_hash: Vec<String> = Vec::new();
    
    for (_, file) in &files {
        if file.is_empty() {
            continue;
        }

        let hash = match calculate_hash_file(file) {
            Ok(hash) => hash,
            Err(err) => {
                error!("Failed to calculate hash file: {}", err);
                return;
            }            
        };

        if hash.is_empty() {
            continue;
        }

        files_hash.push(hash);
    };

    if files_hash.is_empty() {
        return;
    }

    let mut interval = rocket::tokio::time::interval(
        rocket::tokio::time::Duration::from_secs(5),
    );

    loop {
        interval.tick().await;

        for (index, (file_type, file)) in files.iter().enumerate() {

            if file.is_empty() {
                continue;
            }

            let novo_hash = calculate_hash_file(file).unwrap();

            if novo_hash.is_empty() {
                continue;
            }

            if novo_hash != files_hash[index] {
                match file_type {
                    FileType::Data => {
                        let entities = match crate::services::data::load_from_file::load_entities_from_file(PathBuf::from(file)).await {
                            Ok(entities) => entities,
                            Err(err) => {
                                error!("Failed to load entities from file: {}", err);
                                return;
                            }
                        };
                        match data_store.update_entities(entities).await {
                            Ok(entities) => {
                                info!("Successfully updated entities from file {}: {} entities", &file, entities.len());
                            }
                            Err(err) => {
                                error!("Failed to update entities: {}", err);
                                return;
                            }
                        };
                    }
                    FileType::Policy => {
                        let policies = match crate::services::policies::load_from_file::load_policies_from_file(PathBuf::from(file)).await {
                            Ok(policies) => policies,
                            Err(err) => {
                                error!("Failed to load policies from file: {}", err);
                                return;
                            }
                        };
                        match policy_store.update_policies(policies.into_inner()).await {
                            Ok(policies) => {
                                info!("Successfully updated policies from file {}: {} policies", &file, policies.len());
                            }
                            Err(err) => {
                                error!("Failed to update policies: {}", err);
                                return;
                            }
                        };
                    }
                }
                files_hash[index] = novo_hash;
            }
        };
    }
}

pub async fn init(
    data_file_path: String,
    policy_file_path: String, 
    data_store: Arc<MemoryDataStore>, 
    policy_store: Arc<MemoryPolicyStore>
) {
    if data_file_path.is_empty() && policy_file_path.is_empty() {
        return;
    }

    let mut files: Vec<(FileType, String)> = Vec::new();
    files.push((FileType::Data, data_file_path.to_string()));
    files.push((FileType::Policy, policy_file_path.to_string()));
    
    watch_json_file_modify(files, data_store, policy_store).await;
}
    
