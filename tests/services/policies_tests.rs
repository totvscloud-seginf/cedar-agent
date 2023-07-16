use std::str::FromStr;
use std::sync::Arc;
use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

use cedar_policy::PolicyId;

use crate::services::utils::*;
use cedar_agent::policies::memory::MemoryPolicyStore;
use cedar_agent::schemas::policies::PolicyUpdate;
use cedar_agent::PolicyStore;
use cedar_agent::policies::load_from_file::load_policies_from_file;

#[tokio::test]
async fn memory_tests() {
    let store = Arc::new(MemoryPolicyStore::new());

    let policies = store
        .update_policies(vec![approve_all_policy(None)])
        .await
        .unwrap();
    assert_eq!(policies.len(), 1);
    let duplicate_policies = store
        .update_policies(vec![approve_all_policy(None), approve_all_policy(None)])
        .await;
    assert!(duplicate_policies.is_err());
    let error_policies = store.update_policies(vec![parse_error_policy()]).await;
    assert!(error_policies.is_err());

    let created_policy = store
        .create_policy(&approve_admin_policy(Some("admin".to_string())))
        .await
        .unwrap();
    assert_eq!(created_policy.id, "admin".to_string());
    let policy = store.get_policy("admin").await.unwrap();
    assert_eq!(policy.id, "admin".to_string());
    assert_eq!(policy.content, created_policy.content);

    let error_policy = store
        .create_policy(&approve_admin_policy(Some("admin".to_string())))
        .await;
    assert!(error_policy.is_err());
    let error_policy = store.create_policy(&parse_error_policy()).await;
    assert!(error_policy.is_err());

    let policies = store.get_policies().await;
    assert_eq!(policies.len(), 2);

    let updated_policy = store
        .update_policy(
            "test".to_string(),
            PolicyUpdate {
                content: approve_admin_policy(None).content,
            },
        )
        .await
        .unwrap();
    assert_eq!(updated_policy.id, "test".to_string());
    assert_eq!(
        split_content(updated_policy.content.as_str()).1,
        split_content(created_policy.content.as_str()).1
    );

    let error_policy = store
        .update_policy(
            "test".to_string(),
            PolicyUpdate {
                content: parse_error_policy().content,
            },
        )
        .await;
    assert!(error_policy.is_err());

    let deleted_policy = store.delete_policy("test").await.unwrap();
    assert_eq!(deleted_policy.id, "test".to_string());
    let missing_policy = store.get_policy("test").await;
    assert!(missing_policy.is_err());

    assert!(store.delete_policy("test").await.is_err());

    let policy_set = store.policy_set().await;
    assert!(policy_set
        .policy(&PolicyId::from_str("admin").unwrap())
        .is_some());
    assert!(policy_set
        .policy(&PolicyId::from_str("test").unwrap())
        .is_none());
}

#[tokio::test]
async fn test_load_policies_from_file() {
    let temp_file_path = Arc::new(PathBuf::from("test_policies.json"));
    let test_data = r#"[{
        "id": "test",
        "content": "permit(principal in Role::\"Viewer\",action in [Action::\"get\",Action::\"list\"],resource == Document::\"cedar-agent.pdf\");"
    }]"#;
    let mut temp_file = File::create(&*temp_file_path.clone()).unwrap();
    temp_file.write_all(test_data.as_bytes()).unwrap();

    let policies = load_policies_from_file(temp_file_path.clone().to_path_buf()).await.unwrap();
    assert_eq!(policies.len(), 1);
    assert_eq!(policies[0].id, "test".to_string());

    std::fs::remove_file(temp_file_path.clone().to_path_buf()).unwrap();
}