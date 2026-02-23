use reqwest::Client;
use serde::de::{self, Deserializer};
use serde::Deserialize;
use std::time::Duration;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const ENA_RUN_ACCESSION: &str = "ERR194147";
const ENA_STUDY_ACCESSION: &str = "ERP001960";
const ENA_SAMPLE_ACCESSION: &str = "SAMN02422669";

fn ena_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (ena-integration-test)")
        .timeout(Duration::from_secs(15))
        .build()?;
    Ok(client)
}

fn ena_xml_url(accession: &str) -> String {
    format!("https://www.ebi.ac.uk/ena/browser/api/xml/{}", accession)
}

fn ena_json_url(accession: &str) -> String {
    format!("https://www.ebi.ac.uk/ena/browser/api/json/{}", accession)
}

fn ena_portal_run_json_url(accession: &str) -> String {
    format!(
        "https://www.ebi.ac.uk/ena/portal/api/search?result=read_run&query=run_accession=%22{}%22&fields=run_accession,experiment_accession,study_accession,sample_accession,library_strategy,read_count&format=json",
        accession
    )
}

fn ena_portal_study_json_url(accession: &str) -> String {
    format!(
        "https://www.ebi.ac.uk/ena/portal/api/search?result=study&query=secondary_study_accession=%22{}%22&fields=study_accession,study_title&format=json",
        accession
    )
}

fn ena_portal_sample_json_url(accession: &str) -> String {
    format!(
        "https://www.ebi.ac.uk/ena/portal/api/search?result=sample&query=sample_accession=%22{}%22&fields=sample_accession,scientific_name,tax_id&format=json",
        accession
    )
}

async fn fetch_response(client: &Client, url: &str) -> Result<reqwest::Response, AnyError> {
    let response = client.get(url).send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("ENA request failed for URL '{url}' with status {status}: {body}").into());
    }

    Ok(response)
}

async fn fetch_text(client: &Client, url: &str) -> Result<String, AnyError> {
    let response = fetch_response(client, url).await?;
    Ok(response.text().await?)
}

async fn fetch_bytes(client: &Client, url: &str) -> Result<bytes::Bytes, AnyError> {
    let response = fetch_response(client, url).await?;
    Ok(response.bytes().await?)
}

/// Best-effort XML tag extractor for smoke-test assertions only.
fn extract_xml_tag_text(xml: &str, tag: &str) -> Option<String> {
    let start_token = format!("<{tag}");
    let start = xml.find(&start_token)?;
    let after = &xml[start..];
    let open_end = after.find('>')?;
    let content_start = start + open_end + 1;

    let end_token = format!("</{tag}>");
    let rest = &xml[content_start..];
    let rel_end = rest.find(&end_token)?;
    Some(rest[..rel_end].trim().to_string())
}

/// Parses optional integer values where ENA may return null, empty string, a JSON number, or a numeric string.
/// Empty string is treated as None.
fn de_opt_i64_from_str_or_num<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(serde_json::Value::Number(number)) => number
            .as_i64()
            .map(Some)
            .ok_or_else(|| de::Error::custom("Expected i64-compatible number")),
        Some(serde_json::Value::String(text)) => {
            if text.trim().is_empty() {
                Ok(None)
            } else {
                text.parse::<i64>()
                    .map(Some)
                    .map_err(|error| de::Error::custom(format!("Failed parsing i64 from string: {error}")))
            }
        }
        Some(other) => Err(de::Error::custom(format!(
            "Expected null/number/string for optional i64, got {other}"
        ))),
    }
}

fn deserialize_single_record<T>(accession: &str, text: &str, kind: &str) -> Result<T, AnyError>
where
    T: for<'de> serde::Deserialize<'de>,
{
    match serde_json::from_str::<Vec<T>>(text) {
        Ok(mut records) => {
            if records.is_empty() {
                return Err(
                    format!("ENA JSON returned no {kind} records for accession '{accession}'").into(),
                );
            }
            if records.len() > 1 {
                return Err(format!(
                    "ENA JSON returned multiple {kind} records for accession '{accession}'"
                )
                .into());
            }

            Ok(records.remove(0))
        }
        Err(vec_error) => match serde_json::from_str::<T>(text) {
            Ok(record) => Ok(record),
            Err(single_error) => Err(format!(
                "Failed to deserialize ENA {kind} JSON for accession '{accession}'. As Vec<T>: {vec_error}; as T: {single_error}"
            )
            .into()),
        },
    }
}

#[derive(Debug, Deserialize)]
struct EnaRunRecord {
    #[serde(rename = "accession", alias = "run_accession")]
    accession: String,
    #[serde(default, rename = "experiment_accession")]
    experiment_accession: String,
    #[serde(default, rename = "study_accession")]
    study_accession: String,
    #[serde(default, rename = "sample_accession")]
    sample_accession: String,
    #[serde(default, rename = "library_strategy")]
    library_strategy: String,
    #[serde(default, rename = "read_count", deserialize_with = "de_opt_i64_from_str_or_num")]
    read_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct EnaStudyRecord {
    #[serde(rename = "accession", alias = "study_accession")]
    accession: String,
    #[serde(default, rename = "title")]
    title: String,
    #[serde(default, rename = "study_title")]
    study_title: String,
}

#[derive(Debug, Deserialize)]
struct EnaSampleRecord {
    #[serde(rename = "accession", alias = "sample_accession")]
    accession: String,
    #[serde(default)]
    scientific_name: String,
    #[serde(default, rename = "tax_id", deserialize_with = "de_opt_i64_from_str_or_num")]
    tax_id: Option<i64>,
}

async fn ena_fetch_run_json(client: &Client, accession: &str) -> Result<EnaRunRecord, AnyError> {
    let primary_url = ena_json_url(accession);
    let text = match fetch_text(client, &primary_url).await {
        Ok(text) => text,
        Err(_) => {
            let fallback_url = ena_portal_run_json_url(accession);
            fetch_text(client, &fallback_url)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to fetch ENA run JSON for accession '{}' from URL '{}' (fallback URL '{}'): {}",
                        accession, primary_url, fallback_url, error
                    )
                })?
        }
    };

    deserialize_single_record(accession, &text, "run")
}

async fn ena_fetch_study_json(
    client: &Client,
    accession: &str,
) -> Result<EnaStudyRecord, AnyError> {
    let primary_url = ena_json_url(accession);
    let text = match fetch_text(client, &primary_url).await {
        Ok(text) => text,
        Err(_) => {
            let fallback_url = ena_portal_study_json_url(accession);
            fetch_text(client, &fallback_url)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to fetch ENA study JSON for accession '{}' from URL '{}' (fallback URL '{}'): {}",
                        accession, primary_url, fallback_url, error
                    )
                })?
        }
    };

    deserialize_single_record(accession, &text, "study")
}

async fn ena_fetch_sample_json(
    client: &Client,
    accession: &str,
) -> Result<EnaSampleRecord, AnyError> {
    let primary_url = ena_json_url(accession);
    let text = match fetch_text(client, &primary_url).await {
        Ok(text) => text,
        Err(_) => {
            let fallback_url = ena_portal_sample_json_url(accession);
            fetch_text(client, &fallback_url)
                .await
                .map_err(|error| {
                    format!(
                        "Failed to fetch ENA sample JSON for accession '{}' from URL '{}' (fallback URL '{}'): {}",
                        accession, primary_url, fallback_url, error
                    )
                })?
        }
    };

    deserialize_single_record(accession, &text, "sample")
}

#[tokio::test]
async fn ena_run_json_has_core_fields() -> Result<(), AnyError> {
    let client = ena_client()?;
    let run = ena_fetch_run_json(&client, ENA_RUN_ACCESSION).await?;

    assert_eq!(
        run.accession, ENA_RUN_ACCESSION,
        "Expected ENA run accession '{}' in JSON payload, got '{}'",
        ENA_RUN_ACCESSION, run.accession
    );
    assert!(
        !run.experiment_accession.trim().is_empty(),
        "Expected ENA run '{}' JSON to include a non-empty experiment_accession",
        ENA_RUN_ACCESSION
    );
    assert!(
        !run.study_accession.trim().is_empty(),
        "Expected ENA run '{}' JSON to include a non-empty study_accession",
        ENA_RUN_ACCESSION
    );
    assert!(
        !run.sample_accession.trim().is_empty(),
        "Expected ENA run '{}' JSON to include a non-empty sample_accession",
        ENA_RUN_ACCESSION
    );
    assert!(
        !run.library_strategy.trim().is_empty(),
        "Expected ENA run '{}' JSON to include a non-empty library_strategy",
        ENA_RUN_ACCESSION
    );
    if let Some(read_count) = run.read_count {
        assert!(
            read_count > 0,
            "Expected ENA run '{}' read_count to be > 0 when present, got {}",
            ENA_RUN_ACCESSION, read_count
        );
    }

    Ok(())
}

#[tokio::test]
async fn ena_study_json_has_core_fields() -> Result<(), AnyError> {
    let client = ena_client()?;
    let study = ena_fetch_study_json(&client, ENA_STUDY_ACCESSION).await?;

    // ENA may map secondary study accessions to primary study IDs across endpoints.
    assert!(
        !study.accession.trim().is_empty()
            && (study.accession.starts_with("ERP")
                || study.accession.starts_with("PRJ")
                || study.accession == ENA_STUDY_ACCESSION),
        "Expected ENA study JSON for requested accession '{}' to return a non-empty study-like accession (ERP*/PRJ*), got '{}'",
        ENA_STUDY_ACCESSION,
        study.accession
    );
    assert!(
        !study.title.trim().is_empty() || !study.study_title.trim().is_empty(),
        "Expected ENA study '{}' JSON to include non-empty title or study_title",
        ENA_STUDY_ACCESSION
    );

    Ok(())
}

#[tokio::test]
async fn ena_sample_json_has_core_fields() -> Result<(), AnyError> {
    let client = ena_client()?;
    let sample = ena_fetch_sample_json(&client, ENA_SAMPLE_ACCESSION).await?;

    assert_eq!(
        sample.accession, ENA_SAMPLE_ACCESSION,
        "Expected ENA sample accession '{}' in JSON payload, got '{}'",
        ENA_SAMPLE_ACCESSION, sample.accession
    );
    assert!(
        !sample.scientific_name.trim().is_empty(),
        "Expected ENA sample '{}' JSON to include a non-empty scientific_name",
        ENA_SAMPLE_ACCESSION
    );
    if let Some(tax_id) = sample.tax_id {
        assert!(
            tax_id > 0,
            "Expected ENA sample '{}' tax_id to be > 0 when present, got {}",
            ENA_SAMPLE_ACCESSION, tax_id
        );
    }

    Ok(())
}

#[tokio::test]
async fn ena_run_xml_contains_expected_tags() -> Result<(), AnyError> {
    let client = ena_client()?;
    let url = ena_xml_url(ENA_RUN_ACCESSION);
    let xml_bytes = fetch_bytes(&client, &url).await?;
    let xml = String::from_utf8_lossy(&xml_bytes);

    assert!(
        !xml.trim().is_empty(),
        "Expected ENA XML response for '{}' to be non-empty",
        ENA_RUN_ACCESSION
    );
    assert!(
        xml.contains("<RUN"),
        "Expected ENA run XML for '{}' to contain '<RUN'",
        ENA_RUN_ACCESSION
    );
    assert!(
        xml.contains(&format!("accession=\"{}\"", ENA_RUN_ACCESSION)),
        "Expected ENA run XML for '{}' to contain an accession attribute",
        ENA_RUN_ACCESSION
    );
    assert!(
        xml.contains("<EXPERIMENT_REF"),
        "Expected ENA run XML for '{}' to contain '<EXPERIMENT_REF'",
        ENA_RUN_ACCESSION
    );

    let title = extract_xml_tag_text(&xml, "TITLE").unwrap_or_default();
    assert!(
        !title.trim().is_empty(),
        "Expected ENA run XML for '{}' to include a non-empty TITLE tag",
        ENA_RUN_ACCESSION
    );

    Ok(())
}

#[tokio::test]
async fn ena_study_xml_contains_expected_tags() -> Result<(), AnyError> {
    let client = ena_client()?;
    let url = ena_xml_url(ENA_STUDY_ACCESSION);
    let xml = fetch_text(&client, &url).await?;

    assert!(
        !xml.trim().is_empty(),
        "Expected ENA XML response for '{}' to be non-empty",
        ENA_STUDY_ACCESSION
    );
    assert!(
        xml.contains("<STUDY"),
        "Expected ENA study XML for '{}' to contain '<STUDY'",
        ENA_STUDY_ACCESSION
    );
    assert!(
        xml.contains(&format!("accession=\"{}\"", ENA_STUDY_ACCESSION)),
        "Expected ENA study XML for '{}' to contain an accession attribute",
        ENA_STUDY_ACCESSION
    );

    Ok(())
}

#[tokio::test]
async fn ena_sample_xml_contains_expected_tags() -> Result<(), AnyError> {
    let client = ena_client()?;
    let url = ena_xml_url(ENA_SAMPLE_ACCESSION);
    let xml = fetch_text(&client, &url).await?;

    assert!(
        !xml.trim().is_empty(),
        "Expected ENA XML response for '{}' to be non-empty",
        ENA_SAMPLE_ACCESSION
    );
    assert!(
        xml.contains("<SAMPLE"),
        "Expected ENA sample XML for '{}' to contain '<SAMPLE'",
        ENA_SAMPLE_ACCESSION
    );
    assert!(
        xml.contains(&format!("accession=\"{}\"", ENA_SAMPLE_ACCESSION)),
        "Expected ENA sample XML for '{}' to contain an accession attribute",
        ENA_SAMPLE_ACCESSION
    );
    assert!(
        xml.contains("<SCIENTIFIC_NAME>"),
        "Expected ENA sample XML for '{}' to contain '<SCIENTIFIC_NAME>'",
        ENA_SAMPLE_ACCESSION
    );

    let scientific_name = extract_xml_tag_text(&xml, "SCIENTIFIC_NAME").unwrap_or_default();
    assert!(
        !scientific_name.trim().is_empty(),
        "Expected ENA sample XML for '{}' to include a non-empty SCIENTIFIC_NAME tag",
        ENA_SAMPLE_ACCESSION
    );

    Ok(())
}

#[tokio::test]
async fn ena_invalid_accession_returns_error() -> Result<(), AnyError> {
    let client = ena_client()?;
    let bad = "NOT_A_REAL_ENA_ACCESSION";

    let xml_url = ena_xml_url(bad);
    let xml_result = fetch_text(&client, &xml_url).await;
    assert!(
        xml_result.is_err(),
        "Expected ENA XML fetch to fail for invalid accession '{}'",
        bad
    );
    if let Err(error) = xml_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad) || msg.contains("404") || msg.contains("Not Found"),
            "Expected ENA XML error to mention invalid accession or 404-like status for '{}', got '{}'",
            bad,
            msg
        );
    }

    let json_result = ena_fetch_run_json(&client, bad).await;
    assert!(
        json_result.is_err(),
        "Expected ena_fetch_run_json to fail for invalid accession '{}'",
        bad
    );
    if let Err(error) = json_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad) || msg.contains("404") || msg.contains("Not Found") || msg.contains("no run records"),
            "Expected ENA run JSON error to mention invalid accession or no records for '{}', got '{}'",
            bad,
            msg
        );
    }

    let study_result = ena_fetch_study_json(&client, bad).await;
    assert!(
        study_result.is_err(),
        "Expected ena_fetch_study_json to fail for invalid accession '{}'",
        bad
    );
    if let Err(error) = study_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad)
                || msg.contains("404")
                || msg.contains("Not Found")
                || msg.contains("no study records"),
            "Expected ENA study JSON error to mention invalid accession or no records for '{}', got '{}'",
            bad,
            msg
        );
    }

    let sample_result = ena_fetch_sample_json(&client, bad).await;
    assert!(
        sample_result.is_err(),
        "Expected ena_fetch_sample_json to fail for invalid accession '{}'",
        bad
    );
    if let Err(error) = sample_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad)
                || msg.contains("404")
                || msg.contains("Not Found")
                || msg.contains("no sample records"),
            "Expected ENA sample JSON error to mention invalid accession or no records for '{}', got '{}'",
            bad,
            msg
        );
    }

    Ok(())
}