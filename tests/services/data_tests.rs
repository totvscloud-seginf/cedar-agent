use std::path::PathBuf;

use crate::services::utils;

use cedar_agent::data::memory::MemoryDataStore;
use cedar_agent::data::load_from_file::load_entities_from_file;
use cedar_agent::DataStore;

#[tokio::test]
async fn memory_tests() {
    let store = MemoryDataStore::new();

    let entities = store.get_entities().await;
    assert_eq!(entities.len(), 0);
    let updated_entities = store.update_entities(utils::entities()).await.unwrap();
    assert_eq!(updated_entities.len(), 8);

    let error_entities = store.update_entities(utils::parse_error_entities()).await;
    assert!(error_entities.is_err());
    store.delete_entities().await;
    let entities = store.get_entities().await;
    assert_eq!(entities.len(), 0);
}

#[tokio::test]
async fn test_load_entities_from_file() {
    let entities = load_entities_from_file(PathBuf::from("./examples/data.json")).await.unwrap();
    assert_eq!(entities.len(), 12);
}
