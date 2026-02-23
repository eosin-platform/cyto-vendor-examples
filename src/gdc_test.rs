use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const GDC_API_BASE: &str = "https://api.gdc.cancer.gov";
const GDC_FILE_ID: &str = "3845b8bd-cbe0-49cf-a418-a8120f6c23db";
const GDC_PROJECT_ID: &str = "TCGA-BRCA";
const GDC_CASE_SUBMITTER_ID: &str = "TCGA-A1-A0SH-01Z";
const GDC_CASES_URL: &str = "https://api.gdc.cancer.gov/cases";

fn gdc_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (gdc-integration-test)")
        .timeout(Duration::from_secs(15))
        .build()?;
    Ok(client)
}

fn gdc_file_metadata_url(file_id: &str) -> String {
    format!(
        "{GDC_API_BASE}/files/{file_id}?fields=file_id,file_name,data_category,data_type,data_format,access,cases.submitter_id,cases.project.project_id,cases.samples.submitter_id"
    )
}

fn truncate_for_error(text: &str, max_len: usize) -> String {
    let cutoff = text
        .char_indices()
        .nth(max_len)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| text.len());

    if cutoff == text.len() {
        return text.to_owned();
    }
    let snippet = &text[..cutoff];
    format!("{snippet}...[truncated]")
}

fn gdc_data_url(file_id: &str) -> String {
    format!("{GDC_API_BASE}/data/{file_id}")
}

async fn fetch_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, AnyError> {
    let response = client.get(url).send().await?;
    let status = response.status();
    let text = response.text().await?;
    let snippet = truncate_for_error(&text, 4096);

    if !status.is_success() {
        return Err(
            format!("GDC request failed for URL '{url}' with status {status}: {snippet}").into(),
        );
    }

    serde_json::from_str::<T>(&text).map_err(|error| {
        format!(
            "Failed to deserialize GDC JSON response from URL '{url}': {error}; body: {snippet}"
        )
        .into()
    })
}

async fn post_json<T: DeserializeOwned>(
    client: &Client,
    url: &str,
    body: &serde_json::Value,
) -> Result<T, AnyError> {
    let response = client.post(url).json(body).send().await?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let snippet = truncate_for_error(&text, 4096);

    if !status.is_success() {
        return Err(
            format!("GDC request failed for URL '{url}' with status {status}: {snippet}").into(),
        );
    }

    serde_json::from_str::<T>(&text).map_err(|error| {
        format!(
            "Failed to deserialize GDC JSON response from URL '{url}': {error}; body: {snippet}"
        )
        .into()
    })
}

async fn fetch_head(client: &Client, url: &str) -> Result<reqwest::Response, AnyError> {
    let response = client.head(url).send().await?;
    let status = response.status();

    if !status.is_success() {
        return Err(format!("GDC HEAD request failed for URL '{url}' with status {status}").into());
    }

    Ok(response)
}

#[derive(Debug, Deserialize)]
struct GdcFileEnvelope {
    data: GdcFileMeta,
}

#[derive(Debug, Deserialize)]
struct GdcFileMeta {
    #[serde(rename = "file_id")]
    id: String,
    file_name: String,
    data_category: String,
    data_format: String,
    access: String,
    #[serde(default)]
    data_type: String,
    #[serde(default)]
    cases: Vec<GdcCaseMeta>,
}

#[derive(Debug, Deserialize)]
struct GdcCaseMeta {
    submitter_id: String,
    project: ProjectMeta,
    #[serde(default)]
    samples: Vec<GdcSampleMeta>,
}

#[derive(Debug, Deserialize)]
struct GdcSampleMeta {
    submitter_id: String,
}

#[derive(Debug, Deserialize)]
struct ProjectMeta {
    project_id: String,
}

#[derive(Debug, Deserialize)]
struct GdcCasesEnvelope {
    data: GdcCasesData,
}

#[derive(Debug, Deserialize)]
struct GdcCasesData {
    #[serde(default)]
    hits: Vec<GdcCaseWithFiles>,
}

#[derive(Debug, Deserialize)]
struct GdcCaseWithFiles {
    submitter_id: String,
    project: ProjectMeta,
    #[serde(default)]
    files: Vec<GdcAssociatedFile>,
    #[serde(default)]
    submitter_sample_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GdcAssociatedFile {
    #[serde(rename = "file_id")]
    id: String,
    file_name: String,
    data_category: String,
    data_format: String,
    #[serde(default)]
    data_type: String,
}

#[tokio::test]
async fn gdc_file_metadata_basic_fields() -> Result<(), AnyError> {
    let client = gdc_client()?;
    let url = gdc_file_metadata_url(GDC_FILE_ID);
    let envelope: GdcFileEnvelope = fetch_json(&client, &url).await?;
    let file = envelope.data;

    assert_eq!(
        file.id, GDC_FILE_ID,
        "Expected GDC file id '{}' in metadata payload, got '{}'",
        GDC_FILE_ID, file.id
    );
    assert_eq!(
        file.data_format, "SVS",
        "Expected GDC file '{}' data_format to be 'SVS', got '{}'",
        GDC_FILE_ID, file.data_format
    );
    assert_eq!(
        file.access, "open",
        "Expected GDC file '{}' access to be 'open', got '{}'",
        GDC_FILE_ID, file.access
    );
    assert!(
        !file.cases.is_empty(),
        "Expected GDC file '{}' metadata to include at least one associated case",
        GDC_FILE_ID
    );

    assert!(
        file.data_category == "Slide Image"
            || (file.data_category == "Biospecimen" && file.data_type == "Slide Image"),
        "Expected GDC file '{}' to represent a slide image (data_category='Slide Image' or Biospecimen+data_type='Slide Image'), got data_category='{}', data_type='{}'",
        GDC_FILE_ID,
        file.data_category,
        file.data_type
    );

    let project_id = file
        .cases
        .first()
        .map(|case| case.project.project_id.as_str())
        .unwrap_or_default();
    assert_eq!(
        project_id, GDC_PROJECT_ID,
        "Expected GDC file '{}' project_id '{}', got '{}'",
        GDC_FILE_ID, GDC_PROJECT_ID, project_id
    );

    let case_ids: Vec<_> = file
        .cases
        .iter()
        .map(|c| c.submitter_id.as_str())
        .collect();
    let sample_ids: Vec<_> = file
        .cases
        .iter()
        .flat_map(|c| c.samples.iter().map(|s| s.submitter_id.as_str()))
        .collect();

    assert!(
        file.cases.iter().any(|case| {
            case.submitter_id == GDC_CASE_SUBMITTER_ID
                || case
                    .samples
                    .iter()
                    .any(|sample| sample.submitter_id.contains(GDC_CASE_SUBMITTER_ID))
        }),
        "Expected GDC file '{}' metadata to include case submitter id '{}' or a sample submitter containing it; case submitter_ids: {:?}; sample submitter_ids: {:?}",
        GDC_FILE_ID,
        GDC_CASE_SUBMITTER_ID,
        case_ids,
        sample_ids
    );

    assert!(
        file.file_name.to_ascii_lowercase().ends_with(".svs"),
        "Expected GDC file '{}' filename to end with '.svs', got '{}'",
        GDC_FILE_ID,
        file.file_name
    );

    Ok(())
}

#[tokio::test]
async fn gdc_case_has_associated_slide() -> Result<(), AnyError> {
    let client = gdc_client()?;

    let body = json!({
        // Accept match on either case submitter_id or sample submitter_id,
        // because TCGA modeling varies between case-level and sample-level identifiers.
        "filters": {
            "op": "or",
            "content": [
                {
                    "op": "in",
                    "content": {
                        "field": "submitter_id",
                        "value": [GDC_CASE_SUBMITTER_ID]
                    }
                },
                {
                    "op": "in",
                    "content": {
                        "field": "samples.submitter_id",
                        "value": [GDC_CASE_SUBMITTER_ID]
                    }
                }
            ]
        },
        "fields": "submitter_id,submitter_sample_ids,project.project_id,files.file_name,files.data_category,files.data_type,files.data_format,files.file_id",
        "expand": "project,files",
        "format": "JSON",
        "size": 2
    });

    let envelope: GdcCasesEnvelope = post_json(&client, GDC_CASES_URL, &body).await?;
    let hits = envelope.data.hits;

    assert_eq!(
        hits.len(), 1,
        "Expected exactly one GDC case hit for submitter id '{}', got {}",
        GDC_CASE_SUBMITTER_ID,
        hits.len()
    );

    let case = &hits[0];
    assert_eq!(
        case.project.project_id, GDC_PROJECT_ID,
        "Expected GDC case project_id '{}' for submitter '{}', got '{}'",
        GDC_PROJECT_ID, GDC_CASE_SUBMITTER_ID, case.project.project_id
    );
    assert!(
        case.submitter_id.starts_with("TCGA-A1-A0SH")
            || case
                .submitter_sample_ids
                .iter()
                .any(|id| id.contains(GDC_CASE_SUBMITTER_ID)),
        "Expected GDC case payload to include submitter id '{}' in case or sample identifiers; got case submitter_id='{}', sample ids={:?}",
        GDC_CASE_SUBMITTER_ID,
        case.submitter_id,
        case.submitter_sample_ids
    );

    assert!(
        case.files.iter().any(|file| {
            file.id == GDC_FILE_ID
                && file.file_name.to_ascii_lowercase().ends_with(".svs")
                && file.data_format == "SVS"
                && (file.data_category == "Slide Image"
                    || (file.data_category == "Biospecimen" && file.data_type == "Slide Image"))
        }),
        "Expected GDC case '{}' to include known slide file '{}' with slide-image semantics and .svs filename",
        case.submitter_id,
        GDC_FILE_ID
    );

    Ok(())
}

#[tokio::test]
async fn gdc_slide_head_request_valid() -> Result<(), AnyError> {
    let client = gdc_client()?;
    let url = gdc_data_url(GDC_FILE_ID);

    let response = match fetch_head(&client, &url).await {
        Ok(response) => response,
        Err(_) => {
            client
                .get(&url)
                .header(reqwest::header::RANGE, "bytes=0-0")
                .send()
                .await
                .map_err(|error| {
                    format!(
                        "GDC fallback range GET failed for URL '{}' after HEAD rejection: {}",
                        url, error
                    )
                })?
        }
    };

    assert!(
        response.status().is_success(),
        "Expected GDC slide availability check to return success for URL '{}', got status {}",
        url,
        response.status()
    );

    let content_length = response
        .headers()
        .get(reqwest::header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|text| text.parse::<u64>().ok())
        .unwrap_or(0);
    assert!(
        content_length > 0,
        "Expected GDC slide response for '{}' to include content-length > 0, got {}",
        url,
        content_length
    );

    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let lower_content_type = content_type.to_ascii_lowercase();
    assert!(
        lower_content_type.contains("application/octet-stream") || lower_content_type.contains("svs"),
        "Expected GDC slide content-type for '{}' to contain 'application/octet-stream' or 'svs', got '{}'",
        url,
        content_type
    );

    Ok(())
}