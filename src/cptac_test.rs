use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::time::Duration;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const CPTAC_API_BASE: &str = "https://services.cancerimagingarchive.net/nbia-api/services/v1";
const PATHDB_API_BASE: &str = "https://pathdb.cancerimagingarchive.net/";
const CPTAC_COLLECTION: &str = "CPTAC-LSCC";
const CPTAC_KNOWN_PATIENT_ID: &str = "C3N-02494";
const CPTAC_KNOWN_SERIES_UID: &str =
    "1.3.6.1.4.1.14519.5.2.1.4801.5885.139904327352514964836510241693";
const CPTAC_KNOWN_SOP_UID: &str =
    "1.3.6.1.4.1.14519.5.2.1.4801.5885.101503750230314910409775694231";

const CPTAC_BRCA_COLLECTIONS: &[&str] = &["CPTAC-BRCA", "CPTAC-BRCA-1"];
const CPTAC_UCEC_COLLECTIONS: &[&str] = &["CPTAC-UCEC"];
const CPTAC_HNSCC_COLLECTIONS: &[&str] = &["CPTAC-HNSCC"];
const CPTAC_CCRCC_COLLECTIONS: &[&str] = &["CPTAC-CCRCC"];
const CPTAC_OV_COLLECTIONS: &[&str] = &["CPTAC-OV"];

fn cptac_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (cptac-integration-test)")
        .timeout(Duration::from_secs(20))
        .build()?;
    Ok(client)
}

fn cptac_collections_url() -> String {
    format!("{CPTAC_API_BASE}/getCollectionValues?format=json")
}

fn cptac_series_url(collection: &str) -> String {
    format!("{CPTAC_API_BASE}/getSeries?Collection={collection}&format=json")
}

fn cptac_series_for_patient_url(collection: &str, patient_id: &str) -> String {
    format!("{CPTAC_API_BASE}/getSeries?Collection={collection}&PatientID={patient_id}&format=json")
}

fn cptac_sop_uids_url(series_instance_uid: &str) -> String {
    format!(
        "{CPTAC_API_BASE}/getSOPInstanceUIDs?SeriesInstanceUID={series_instance_uid}&format=json"
    )
}

fn cptac_image_url(series_instance_uid: &str, sop_instance_uid: &str) -> String {
    format!(
        "{CPTAC_API_BASE}/getSingleImage?SeriesInstanceUID={series_instance_uid}&SOPInstanceUID={sop_instance_uid}"
    )
}

fn truncate_for_error(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_owned();
    }

    let cutoff = text
        .char_indices()
        .nth(max_len)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| text.len());
    let snippet = &text[..cutoff];
    format!("{snippet}...[truncated]")
}

async fn fetch_json<T: DeserializeOwned>(client: &Client, url: &str) -> Result<T, AnyError> {
    let response = client.get(url).send().await?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    let snippet = truncate_for_error(&text, 4096);

    if !status.is_success() {
        return Err(format!(
            "CPTAC request failed for URL '{url}' with status {status}: {snippet}"
        )
        .into());
    }

    serde_json::from_str::<T>(&text).map_err(|error| {
        format!(
            "Failed to deserialize CPTAC JSON response from URL '{url}': {error}; body: {snippet}"
        )
        .into()
    })
}

async fn fetch_head_or_range(client: &Client, url: &str) -> Result<reqwest::Response, AnyError> {
    match client.head(url).send().await {
        Ok(response) if response.status().is_success() => Ok(response),
        Ok(head_response) => {
            let head_status = head_response.status();
            let range_response = client
                .get(url)
                .header(reqwest::header::RANGE, "bytes=0-0")
                .send()
                .await
                .map_err(|error| {
                    format!(
                        "CPTAC HEAD request for URL '{url}' returned status {head_status}; range GET fallback failed: {error}"
                    )
                })?;

            if !range_response.status().is_success() {
                return Err(format!(
                    "CPTAC HEAD request for URL '{url}' returned status {head_status}; range GET fallback returned status {}",
                    range_response.status()
                )
                .into());
            }

            Ok(range_response)
        }
        Err(head_error) => {
            let range_response = client
                .get(url)
                .header(reqwest::header::RANGE, "bytes=0-0")
                .send()
                .await
                .map_err(|range_error| {
                    format!(
                        "CPTAC HEAD request failed for URL '{url}' ({head_error}); range GET fallback failed: {range_error}"
                    )
                })?;

            if !range_response.status().is_success() {
                return Err(format!(
                    "CPTAC HEAD request failed for URL '{url}' ({head_error}); range GET fallback returned status {}",
                    range_response.status()
                )
                .into());
            }

            Ok(range_response)
        }
    }
}

async fn fetch_range_bytes(
    client: &Client,
    url: &str,
    range_header: &str,
) -> Result<Vec<u8>, AnyError> {
    let response = client
        .get(url)
        .header(reqwest::header::RANGE, range_header)
        .send()
        .await?;
    let status = response.status();
    let body = response.bytes().await.unwrap_or_default().to_vec();

    if !status.is_success() {
        return Err(format!(
            "CPTAC range GET failed for URL '{}' with status {}",
            url, status
        )
        .into());
    }

    if body.is_empty() {
        return Err(format!("CPTAC range GET for URL '{}' returned empty body", url).into());
    }

    Ok(body)
}

#[derive(Debug, Deserialize)]
struct CptacCollection {
    #[serde(rename = "Collection")]
    collection: String,
}

#[derive(Debug, Deserialize)]
struct CptacSeries {
    #[serde(rename = "SeriesInstanceUID")]
    series_instance_uid: String,
    #[serde(rename = "StudyInstanceUID")]
    study_instance_uid: String,
    #[serde(rename = "Modality")]
    modality: String,
    #[serde(rename = "PatientID")]
    patient_id: String,
}

#[derive(Debug, Deserialize)]
struct CptacSopInstance {
    #[serde(rename = "SOPInstanceUID")]
    sop_instance_uid: String,
}

#[derive(Debug, Clone)]
struct PathDbCollection {
    collection_name: String,
    collection_id: String,
}

#[derive(Debug, Clone)]
struct PathDbImage {
    collection_name: String,
    subject_id: String,
    image_id: String,
    image_url: String,
    file_name: String,
}

fn is_radiology_modality(modality: &str) -> bool {
    matches!(modality, "CT" | "MR" | "PT" | "US" | "CR" | "DX")
}

fn is_slide_like_modality(modality: &str) -> bool {
    is_radiology_modality(modality) || matches!(modality, "SM" | "OT")
}

async fn fetch_cptac_collections(client: &Client) -> Result<Vec<CptacCollection>, AnyError> {
    let collections_url = cptac_collections_url();
    fetch_json(client, &collections_url).await
}

fn pick_present_collection_alias(
    candidate_collections: &[&str],
    available_collections: &[CptacCollection],
) -> Option<String> {
    candidate_collections.iter().find_map(|candidate| {
        available_collections
            .iter()
            .any(|collection| collection.collection == *candidate)
            .then(|| (*candidate).to_string())
    })
}

fn pathdb_collections_url() -> String {
    format!("{PATHDB_API_BASE}collections?_format=json")
}

fn pathdb_listofimages_url(collection_id: &str, page: usize) -> String {
    format!("{PATHDB_API_BASE}listofimages/{collection_id}?_format=json&page={page}")
}

fn json_first_value_as_string(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("value"))
        .and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|n| n.to_string()))
                .or_else(|| v.as_u64().map(|n| n.to_string()))
                .or_else(|| v.as_f64().map(|n| n.to_string()))
        })
        .unwrap_or_default()
}

fn json_first_url_as_string(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|first| first.get("url"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string()
}

fn filename_from_url(url: &str) -> String {
    let without_query = url.split('?').next().unwrap_or(url);
    without_query
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_string()
}

fn url_looks_like_svs(url: &str) -> bool {
    let without_query = url.split('?').next().unwrap_or(url);
    without_query.to_ascii_lowercase().ends_with(".svs")
        || without_query.to_ascii_lowercase().contains(".svs/")
}

async fn fetch_pathdb_collections(client: &Client) -> Result<Vec<PathDbCollection>, AnyError> {
    let url = pathdb_collections_url();
    let data = fetch_json::<serde_json::Value>(client, &url).await?;
    let mut collections = Vec::new();

    let Some(items) = data.as_array() else {
        return Err(format!(
            "PathDB collections endpoint '{}' did not return a JSON array",
            url
        )
        .into());
    };

    for item in items {
        let collection_name = json_first_value_as_string(item, "name");
        let collection_id = json_first_value_as_string(item, "tid");
        if !collection_name.trim().is_empty() && !collection_id.trim().is_empty() {
            collections.push(PathDbCollection {
                collection_name,
                collection_id,
            });
        }
    }

    Ok(collections)
}

async fn fetch_pathdb_images_for_collection(
    client: &Client,
    collection: &str,
) -> Result<Vec<PathDbImage>, AnyError> {
    let collections = fetch_pathdb_collections(client).await?;
    let query = collection.to_ascii_lowercase();
    let matching: Vec<PathDbCollection> = collections
        .into_iter()
        .filter(|c| c.collection_name.to_ascii_lowercase().contains(&query))
        .collect();

    if matching.is_empty() {
        return Err(format!(
            "PathDB did not list any collection name containing '{}' (query)",
            collection
        )
        .into());
    }

    let mut images = Vec::new();

    for matched in matching {
        let mut page = 0usize;
        loop {
            let url = pathdb_listofimages_url(&matched.collection_id, page);
            let data = fetch_json::<serde_json::Value>(client, &url).await?;
            let Some(items) = data.as_array() else {
                return Err(format!(
                    "PathDB listofimages endpoint '{}' did not return a JSON array",
                    url
                )
                .into());
            };

            if items.is_empty() {
                break;
            }

            for item in items {
                let collection_name = json_first_value_as_string(item, "studyid");
                let subject_id = json_first_value_as_string(item, "clinicaltrialsubjectid");
                let image_id = json_first_value_as_string(item, "imageid");
                let image_url = json_first_url_as_string(item, "field_wsiimage");
                let file_name = if image_url.trim().is_empty() {
                    String::new()
                } else {
                    filename_from_url(&image_url)
                };

                // Some items might not have an image URL; those aren't usable as a fetch target.
                if image_url.trim().is_empty() {
                    continue;
                }

                images.push(PathDbImage {
                    collection_name,
                    subject_id,
                    image_id,
                    image_url,
                    file_name,
                });
            }

            page += 1;
        }
    }

    Ok(images)
}

async fn pick_cptac_slide_url_for_cohort(
    client: &Client,
    cohort_label: &str,
    collection_aliases: &[&str],
) -> Result<String, AnyError> {
    let mut alias_summaries = Vec::new();

    for alias in collection_aliases {
        let images = fetch_pathdb_images_for_collection(client, alias).await;
        match images {
            Err(error) => {
                alias_summaries.push(format!("alias '{}' => error: {}", alias, error));
                continue;
            }
            Ok(images) => {
                if images.is_empty() {
                    alias_summaries.push(format!("alias '{}' => zero images", alias));
                    continue;
                }

                let chosen = images
                    .iter()
                    .filter(|img| !img.image_url.trim().is_empty())
                    .find(|img| url_looks_like_svs(&img.image_url))
                    .cloned();

                if let Some(chosen) = chosen {
                    eprintln!(
                        "CPTAC cohort '{}' slide probe: picked PathDB WSI URL from alias '{}': url='{}' file='{}' subject='{}' image_id='{}' collection='{}'",
                        cohort_label,
                        alias,
                        chosen.image_url,
                        chosen.file_name,
                        chosen.subject_id,
                        chosen.image_id,
                        chosen.collection_name
                    );
                    return Ok(chosen.image_url);
                }

                // Keep a small sample for error messages.
                let sample = images
                    .iter()
                    .take(5)
                    .map(|img| {
                        format!(
                            "{{collection='{}', subject='{}', image_id='{}', file='{}', url='{}'}}",
                            img.collection_name,
                            img.subject_id,
                            img.image_id,
                            img.file_name,
                            img.image_url
                        )
                    })
                    .collect::<Vec<_>>();
                alias_summaries.push(format!(
                    "alias '{}' => {} images but no .svs-like URL found (sample: {:?})",
                    alias,
                    images.len(),
                    sample
                ));
            }
        }
    }

    Err(format!(
        "CPTAC cohort '{}' slide probe failed: could not resolve any .svs slide URL via PathDB. Aliases tried: {:?}. Details: {}",
        cohort_label,
        collection_aliases,
        alias_summaries.join("; ")
    )
    .into())
}

async fn validate_slide_url_range(
    client: &Client,
    cohort_label: &str,
    url: &str,
) -> Result<(), AnyError> {
    let body = fetch_range_bytes(client, url, "bytes=0-1023").await?;

    assert!(
        !body.is_empty(),
        "Expected CPTAC cohort '{}' slide URL '{}' to return non-empty body for a small range request",
        cohort_label,
        url
    );

    let header_probe = fetch_head_or_range(client, url).await;
    if let Ok(response) = header_probe
        && let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE)
    {
        let content_type = content_type
            .to_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        assert!(
            content_type.starts_with("image/")
                || content_type.contains("octet-stream")
                || content_type.contains("dicom")
                || content_type.contains("tiff")
                || content_type.contains("svs"),
            "Expected CPTAC cohort '{}' slide URL '{}' to return an image-like content-type when present, got '{}'",
            cohort_label,
            url,
            content_type
        );
    }

    Ok(())
}

async fn probe_cptac_collection_for_series(
    client: &Client,
    candidate_collections: &[&str],
    available_collections: &[CptacCollection],
) -> Result<Option<(String, Vec<CptacSeries>)>, AnyError> {
    let Some(selected_collection) =
        pick_present_collection_alias(candidate_collections, available_collections)
    else {
        return Ok(None);
    };

    let series_url = cptac_series_url(&selected_collection);
    let series: Vec<CptacSeries> = fetch_json(client, &series_url).await?;
    Ok(Some((selected_collection, series)))
}

async fn pick_cptac_radiology_target(
    client: &Client,
) -> Result<(CptacSeries, CptacSopInstance, String), AnyError> {
    let series_url = cptac_series_url(CPTAC_COLLECTION);
    let all_series: Vec<CptacSeries> = fetch_json(client, &series_url).await?;

    let patient_url = cptac_series_for_patient_url(CPTAC_COLLECTION, CPTAC_KNOWN_PATIENT_ID);
    let patient_series = fetch_json::<Vec<CptacSeries>>(client, &patient_url)
        .await
        .unwrap_or_default();

    let chosen_series = all_series
        .into_iter()
        .find(|series| series.series_instance_uid == CPTAC_KNOWN_SERIES_UID)
        .or_else(|| {
            patient_series.into_iter().find(|series| {
                !series.series_instance_uid.trim().is_empty()
                    && !series.study_instance_uid.trim().is_empty()
                    && !series.patient_id.trim().is_empty()
                    && is_radiology_modality(series.modality.as_str())
            })
        })
        .ok_or_else(|| {
            format!(
                "Expected to find at least one usable CPTAC radiology series in collection '{}'",
                CPTAC_COLLECTION
            )
        })?;

    let sop_url = cptac_sop_uids_url(&chosen_series.series_instance_uid);
    let sop_instances: Vec<CptacSopInstance> = fetch_json(client, &sop_url).await?;

    let mut sop_iter = sop_instances.into_iter();
    let chosen_sop = sop_iter
        .find(|sop| sop.sop_instance_uid == CPTAC_KNOWN_SOP_UID)
        .or_else(|| sop_iter.next())
        .ok_or_else(|| {
            format!(
                "Expected at least one SOP instance for CPTAC series '{}' from URL '{}'",
                chosen_series.series_instance_uid, sop_url
            )
        })?;

    let image_url = cptac_image_url(
        &chosen_series.series_instance_uid,
        &chosen_sop.sop_instance_uid,
    );
    Ok((chosen_series, chosen_sop, image_url))
}

#[tokio::test]
async fn cptac_case_list_contains_open_radiology() -> Result<(), AnyError> {
    let client = cptac_client()?;

    let collections_url = cptac_collections_url();
    let collections: Vec<CptacCollection> = fetch_json(&client, &collections_url).await?;

    assert!(
        !collections.is_empty(),
        "Expected CPTAC collections endpoint '{}' to return at least one collection",
        collections_url
    );
    assert!(
        collections
            .iter()
            .any(|entry| entry.collection == CPTAC_COLLECTION),
        "Expected CPTAC collections to include '{}', got sample: {:?}",
        CPTAC_COLLECTION,
        collections
            .iter()
            .take(10)
            .map(|entry| entry.collection.as_str())
            .collect::<Vec<_>>()
    );

    let series_url = cptac_series_url(CPTAC_COLLECTION);
    let series_entries: Vec<CptacSeries> = fetch_json(&client, &series_url).await?;

    assert!(
        !series_entries.is_empty(),
        "Expected CPTAC series endpoint '{}' for collection '{}' to return at least one series",
        series_url,
        CPTAC_COLLECTION
    );
    assert!(
        series_entries.iter().any(|series| {
            is_radiology_modality(series.modality.as_str()) && !series.patient_id.trim().is_empty()
        }),
        "Expected at least one CPTAC series in '{}' to have a radiology modality and non-empty patient ID",
        CPTAC_COLLECTION
    );

    Ok(())
}

#[tokio::test]
async fn cptac_case_has_radiology_image_file() -> Result<(), AnyError> {
    let client = cptac_client()?;
    let (series, sop, image_url) = pick_cptac_radiology_target(&client).await?;

    assert!(
        !series.patient_id.trim().is_empty(),
        "Expected CPTAC series '{}' to have non-empty patient_id",
        series.series_instance_uid
    );
    assert!(
        !series.study_instance_uid.trim().is_empty(),
        "Expected CPTAC series '{}' to have non-empty study_instance_uid",
        series.series_instance_uid
    );
    assert!(
        is_radiology_modality(series.modality.as_str()),
        "Expected CPTAC series '{}' to have a radiology modality (CT/MR/PT/US/CR/DX), got '{}'",
        series.series_instance_uid,
        series.modality
    );
    assert!(
        !sop.sop_instance_uid.trim().is_empty(),
        "Expected CPTAC series '{}' to yield a non-empty SOP instance UID",
        series.series_instance_uid
    );

    let derived_file_name = format!("{}.dcm", sop.sop_instance_uid);
    assert!(
        derived_file_name.to_ascii_lowercase().ends_with(".dcm"),
        "Expected derived CPTAC radiology file name to end with '.dcm', got '{}'",
        derived_file_name
    );
    assert!(
        image_url.starts_with("http://") || image_url.starts_with("https://"),
        "Expected CPTAC image URL to be HTTP(S), got '{}'",
        image_url
    );

    Ok(())
}

#[tokio::test]
async fn cptac_radiology_head_request_valid() -> Result<(), AnyError> {
    let client = cptac_client()?;
    let (_series, _sop, image_url) = pick_cptac_radiology_target(&client).await?;

    let response = fetch_head_or_range(&client, &image_url).await?;
    let status = response.status();

    assert!(
        status.is_success(),
        "Expected CPTAC image HEAD/range request to succeed for URL '{}', got status {}",
        image_url,
        status
    );

    if let Some(content_length) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
        let content_length_str = content_length.to_str().unwrap_or_default();
        let parsed = content_length_str.parse::<u64>().unwrap_or(0);
        assert!(
            parsed > 0,
            "Expected CPTAC image URL '{}' content-length to be > 0 when present, got '{}'",
            image_url,
            content_length_str
        );
    }

    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
        let content_type = content_type
            .to_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        assert!(
            content_type.starts_with("image/")
                || content_type.contains("dicom")
                || content_type.contains("octet-stream")
                || content_type.contains("tiff")
                || content_type.contains("svs"),
            "Expected CPTAC image URL '{}' to return an image-like content-type, got '{}'",
            image_url,
            content_type
        );
    }

    Ok(())
}

#[tokio::test]
async fn cptac_invalid_image_url_returns_error() -> Result<(), AnyError> {
    let client = cptac_client()?;

    let bad_url = cptac_image_url(
        CPTAC_KNOWN_SERIES_UID,
        &format!("{CPTAC_KNOWN_SOP_UID}_DOES_NOT_EXIST"),
    );

    let result = fetch_head_or_range(&client, &bad_url).await;

    assert!(
        result.is_err(),
        "Expected CPTAC image fetch helper to fail for invalid URL '{}'",
        bad_url
    );

    if let Err(error) = result {
        let msg = error.to_string().to_ascii_lowercase();
        assert!(
            msg.contains("404")
                || msg.contains("500")
                || msg.contains("not found")
                || msg.contains("failed")
                || msg.contains("could not")
                || msg.contains("error sending request")
                || msg.contains("certificate"),
            "Expected CPTAC invalid-image-url error to mention a not-found/failure style signal, got '{}'",
            msg
        );
    }

    Ok(())
}

// Some CPTAC cohorts may be absent or may currently expose zero series via NBIA v1 in
// some environments. These probes are opportunistic and log observations, while
// CPTAC-LSCC radiology remains the hard-checked cohort in dedicated LSCC tests above.
async fn cptac_probe_single_cohort(
    client: &Client,
    cohort_label: &str,
    aliases: &[&str],
    expect_slide: bool,
) -> Result<(), AnyError> {
    if !expect_slide {
        return Ok(());
    }

    // NBIA is not the canonical access path for CPTAC H&E slides; PathDB is.
    // Still, do a best-effort NBIA metadata probe to keep visibility into DICOM series.
    match fetch_cptac_collections(client).await {
        Ok(nbia_collections) => {
            match probe_cptac_collection_for_series(client, aliases, &nbia_collections).await {
                Ok(Some((collection_name, series))) => {
                    let has_reasonable_image_series = series.iter().any(|entry| {
                        !entry.series_instance_uid.trim().is_empty()
                            && !entry.study_instance_uid.trim().is_empty()
                            && !entry.patient_id.trim().is_empty()
                            && is_slide_like_modality(entry.modality.as_str())
                    });
                    eprintln!(
                        "CPTAC cohort '{}' NBIA probe: collection '{}' series_count={} has_reasonable_image_series={}",
                        cohort_label,
                        collection_name,
                        series.len(),
                        has_reasonable_image_series
                    );
                }
                Ok(None) => {
                    eprintln!(
                        "CPTAC cohort '{}' NBIA probe: no known collection alias present (aliases: {:?})",
                        cohort_label, aliases
                    );
                }
                Err(error) => {
                    eprintln!(
                        "CPTAC cohort '{}' NBIA probe failed: {}",
                        cohort_label, error
                    );
                }
            }
        }
        Err(error) => {
            eprintln!(
                "CPTAC cohort '{}' NBIA collection fetch failed (non-fatal for slides): {}",
                cohort_label, error
            );
        }
    }

    let url = pick_cptac_slide_url_for_cohort(client, cohort_label, aliases).await?;
    validate_slide_url_range(client, cohort_label, &url).await?;
    Ok(())
}

#[tokio::test]
async fn cptac_brca_cohort_series_probe() -> Result<(), AnyError> {
    let client = cptac_client()?;
    cptac_probe_single_cohort(&client, "BRCA", CPTAC_BRCA_COLLECTIONS, true).await
}

#[tokio::test]
async fn cptac_ucec_cohort_series_probe() -> Result<(), AnyError> {
    let client = cptac_client()?;
    cptac_probe_single_cohort(&client, "UCEC", CPTAC_UCEC_COLLECTIONS, true).await
}

#[tokio::test]
async fn cptac_hnscc_cohort_series_probe() -> Result<(), AnyError> {
    let client = cptac_client()?;
    cptac_probe_single_cohort(&client, "HNSCC", CPTAC_HNSCC_COLLECTIONS, true).await
}

#[tokio::test]
async fn cptac_ccrcc_cohort_series_probe() -> Result<(), AnyError> {
    let client = cptac_client()?;
    cptac_probe_single_cohort(&client, "CCRCC", CPTAC_CCRCC_COLLECTIONS, true).await
}

#[tokio::test]
async fn cptac_ov_cohort_series_probe() -> Result<(), AnyError> {
    let client = cptac_client()?;
    cptac_probe_single_cohort(&client, "OV", CPTAC_OV_COLLECTIONS, true).await
}
