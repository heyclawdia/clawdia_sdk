use clawdia_sdk::{
    core::{CanonicalToolName, ProviderArgumentStore},
    stores::FileStoreBundle,
};

fn main() -> Result<(), clawdia_sdk::core::AgentError> {
    let root = std::env::temp_dir().join("clawdia-sdk-example-file-store");
    let bundle = FileStoreBundle::new(root);
    let store = bundle.provider_arguments();
    let content_ref = store
        .store_provider_arguments(
            "provider.example",
            "call_example",
            &CanonicalToolName::new("workspace_read"),
            r#"{"path":"README.md"}"#,
        )?
        .expect("argument content ref");
    let loaded = store.load_provider_arguments_json(&content_ref)?;
    println!("{} {}", content_ref.as_str(), loaded["path"]);
    Ok(())
}
