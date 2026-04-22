use std::fs;
use std::path::Path;

use colored::Colorize;
use openapi_forge::Spec;
use terraform_forge::{
    ProviderSpec, ResourceSpec, generate_provider, generate_resource, generate_test,
};

use crate::helpers_template;

pub fn run(
    spec_path: &Path,
    resources_dir: &Path,
    output_dir: &Path,
    provider_path: Option<&Path>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{} Loading OpenAPI spec from {}",
        "=>".blue().bold(),
        spec_path.display()
    );
    let api = Spec::load(spec_path)?;

    let provider = if let Some(p) = provider_path {
        ProviderSpec::load(p)?
    } else {
        let provider_toml = resources_dir
            .parent()
            .unwrap_or(resources_dir)
            .join("provider.toml");
        if provider_toml.exists() {
            ProviderSpec::load(&provider_toml)?
        } else {
            return Err("No provider.toml found. Use --provider to specify one.".into());
        }
    };

    let defaults = provider.defaults.clone();

    let resource_files = find_toml_files(resources_dir)?;
    println!(
        "{} Found {} resource specs",
        "=>".blue().bold(),
        resource_files.len()
    );

    let resources_out = output_dir.join("resources");
    let provider_out = output_dir.join("provider");
    let tests_out = output_dir.join("resources");
    fs::create_dir_all(&resources_out)?;
    fs::create_dir_all(&provider_out)?;

    let mut type_names = Vec::new();
    let mut tf_names = Vec::new();
    let mut test_count = 0;
    let mut skipped = 0;

    for file in &resource_files {
        let resource = ResourceSpec::load(file)?;

        if let Err(e) = resource.validate(&api) {
            eprintln!("{} {}: {e}", "warning:".yellow().bold(), file.display());
            skipped += 1;
            continue;
        }

        println!("  {} Generating {}", "->".green(), resource.resource.name);

        let generated =
            generate_resource(&resource, &api, &defaults, &provider.provider.sdk_import)?;

        type_names.push(generated.resource_type_name.clone());
        tf_names.push(resource.resource.name.clone());

        // Check for override file
        let override_path = output_dir.join("overrides").join(&generated.file_name);
        if override_path.exists() {
            println!(
                "  {} Override exists, skipping: {}",
                "!!".yellow(),
                generated.file_name
            );
            fs::copy(&override_path, resources_out.join(&generated.file_name))?;
        } else {
            fs::write(resources_out.join(&generated.file_name), &generated.go_code)?;
        }

        // Generate acceptance test scaffold
        let test = generate_test(&resource);
        fs::write(tests_out.join(&test.file_name), &test.go_code)?;
        test_count += 1;
    }

    if skipped > 0 {
        eprintln!(
            "{} {skipped} resource(s) skipped due to validation errors",
            "warning:".yellow().bold()
        );
    }

    // Generate provider.go
    println!(
        "  {} Generating provider.go ({} resources)",
        "->".green(),
        type_names.len()
    );
    let data_source_names: Vec<String> = Vec::new();
    let provider_code = generate_provider(&provider, &type_names, &tf_names, &data_source_names);
    fs::write(provider_out.join("provider.go"), &provider_code)?;

    // Generate common helpers
    fs::write(resources_out.join("helpers.go"), helpers_template::HELPERS_GO)?;

    println!(
        "\n{} Generated {} resources + {} tests + provider.go in {}",
        "done".green().bold(),
        type_names.len(),
        test_count,
        output_dir.display()
    );

    Ok(())
}

fn find_toml_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in glob::glob(&format!("{}/**/*.toml", dir.display()))? {
            files.push(entry?);
        }
    }
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_minimal_spec(dir: &Path) -> std::path::PathBuf {
        let spec = r#"
openapi: "3.0.0"
info: { title: Test, version: "1.0" }
paths:
  /create-secret:
    post:
      operationId: createSecret
      requestBody:
        content:
          application/json:
            schema:
              $ref: '#/components/schemas/createSecret'
      responses:
        "200": { description: ok }
  /describe-item:
    post:
      operationId: describeItem
      responses:
        "200": { description: ok }
  /delete-item:
    post:
      operationId: deleteItem
      responses:
        "200": { description: ok }
components:
  schemas:
    createSecret:
      type: object
      required: [name, value]
      properties:
        name: { type: string }
        value: { type: string }
    describeItem:
      type: object
      properties:
        name: { type: string }
    deleteItem:
      type: object
      properties:
        name: { type: string }
"#;
        let path = dir.join("spec.yaml");
        fs::write(&path, spec).unwrap();
        path
    }

    fn write_provider_toml(dir: &Path) -> std::path::PathBuf {
        let toml_content = r#"
[provider]
name = "test"
description = "Test provider"
version = "0.1.0"
sdk_import = "github.com/test/sdk"

[auth]
token_field = "token"
env_var = "TEST_TOKEN"
gateway_url_field = "url"
gateway_env_var = "TEST_URL"

[defaults]
skip_fields = ["token"]
"#;
        let path = dir.join("provider.toml");
        fs::write(&path, toml_content).unwrap();
        path
    }

    fn write_resource_toml(dir: &Path) -> std::path::PathBuf {
        let toml_content = r#"
[resource]
name = "test_secret"
description = "Test secret"
category = "secret"

[crud]
create_endpoint = "/create-secret"
create_schema = "createSecret"
read_endpoint = "/describe-item"
read_schema = "describeItem"
delete_endpoint = "/delete-item"
delete_schema = "deleteItem"

[identity]
id_field = "name"
force_new_fields = ["name"]

[fields]
token = { skip = true }
"#;
        let path = dir.join("secret.toml");
        fs::write(&path, toml_content).unwrap();
        path
    }

    #[test]
    fn test_find_toml_files_empty_dir() {
        let dir = TempDir::new().unwrap();
        let files = find_toml_files(dir.path()).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_find_toml_files_nested() {
        let dir = TempDir::new().unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir_all(&sub).unwrap();
        fs::write(sub.join("test.toml"), "[resource]\nname = \"x\"").unwrap();
        let files = find_toml_files(dir.path()).unwrap();
        assert_eq!(files.len(), 1);
    }

    #[test]
    fn test_end_to_end_generate() {
        let dir = TempDir::new().unwrap();
        let spec_path = write_minimal_spec(dir.path());
        let provider_path = write_provider_toml(dir.path());
        let resources_dir = dir.path().join("resources");
        fs::create_dir_all(&resources_dir).unwrap();
        write_resource_toml(&resources_dir);
        let output_dir = dir.path().join("output");

        let result = run(&spec_path, &resources_dir, &output_dir, Some(&provider_path));
        assert!(result.is_ok(), "generate failed: {:?}", result.err());

        // Verify output files exist
        assert!(output_dir.join("provider/provider.go").exists());
        assert!(output_dir.join("resources/helpers.go").exists());
        assert!(output_dir.join("resources/resource_test_secret.go").exists());
        assert!(
            output_dir
                .join("resources/resource_test_secret_test.go")
                .exists()
        );
    }
}
