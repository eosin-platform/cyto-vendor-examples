//! GTEx histology integration tests.
//!
//! These tests exercise GTEx *field-of-view* (FOV) histology imagery end-to-end
//! by:
//! 1) Discovering a real histology record via the GTEx Portal **v2** public API.
//! 2) Deriving a DeepZoom descriptor URL (`.dzi`) and a tile URL.
//! 3) Fetching a small byte range from the tile (HEAD and/or Range GET).
//!
//! Notes:
//! - We intentionally do not use GTEx API v1 (deprecated).
//! - We intentionally do not use checked-in fixtures or env vars.

use reqwest::Client;
use serde::Deserialize;
use std::time::Duration;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

fn gtex_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (gtex-fov-integration-test)")
        .timeout(Duration::from_secs(20))
        .build()?;
    Ok(client)
}

const GTEX_PORTAL_V2_BASE_URL: &str = "https://gtexportal.org/api/v2";

// This value is used by the GTEx Portal frontend (DeepZoom / OpenSeadragon)
// to serve DeepZoom descriptors and tiles for GTEx histology images.
const GTEX_HISTOLOGY_DZI_URL_ROOT: &str = "https://gtexportal.org/openslide/gtexhip/";

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
                        "GTEx HEAD request for URL '{url}' returned status {head_status}; range GET fallback failed: {error}"
                    )
                })?;

            if !range_response.status().is_success() {
                return Err(format!(
                    "GTEx HEAD request for URL '{url}' returned status {head_status}; range GET fallback returned status {}",
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
                        "GTEx HEAD request failed for URL '{url}' ({head_error}); range GET fallback failed: {range_error}"
                    )
                })?;

            if !range_response.status().is_success() {
                return Err(format!(
                    "GTEx HEAD request failed for URL '{url}' ({head_error}); range GET fallback returned status {}",
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
            "GTEx range GET failed for URL '{}' with status {}",
            url, status
        )
        .into());
    }

    if body.is_empty() {
        return Err(format!("GTEx range GET for URL '{}' returned empty body", url).into());
    }

    Ok(body)
}

#[derive(Debug, Clone, Deserialize)]
struct GtexPaginatedResponse<T> {
    data: Vec<T>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GtexHistologySample {
    histology_image_id: String,
    subject_id: String,
    tissue_site_detail: String,
}

#[derive(Debug, Clone)]
struct GtexResolvedFov {
    subject_id: String,
    histology_image_id: String,
    tissue_site_detail: String,
    dzi_url: String,
    tile_url: String,
}

fn extract_dzi_format(dzi_xml: &str) -> Result<String, AnyError> {
    // Example:
    // <Image Format="jpeg" Overlap="1" TileSize="254" ...>
    let marker = "Format=\"";
    let start = dzi_xml
        .find(marker)
        .ok_or_else(|| format!("DeepZoom descriptor missing {marker} attribute: '{dzi_xml}'"))?
        + marker.len();

    let rest = &dzi_xml[start..];
    let end = rest
        .find('"')
        .ok_or_else(|| format!("DeepZoom descriptor has unterminated Format attribute: '{dzi_xml}'"))?;

    let format = rest[..end].trim();
    if format.is_empty() {
        return Err(format!("DeepZoom descriptor has empty Format attribute: '{dzi_xml}'").into());
    }

    Ok(format.to_ascii_lowercase())
}

async fn resolve_gtex_fov_from_v2_api(client: &Client) -> Result<GtexResolvedFov, AnyError> {
    // Deterministic: always use the first record from the first page.
    let api_url = format!(
        "{GTEX_PORTAL_V2_BASE_URL}/histology/image?itemsPerPage=1&page=0"
    );

    let response = client.get(&api_url).send().await?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "GTEx v2 histology API request failed: GET '{api_url}' returned status {status}; body: {body}"
        )
        .into());
    }

    let parsed = serde_json::from_str::<GtexPaginatedResponse<GtexHistologySample>>(&body)
        .map_err(|error| {
            format!(
                "Failed to parse GTEx v2 histology API response from '{api_url}': {error}; body: {body}"
            )
        })?;

    let sample = parsed.data.into_iter().next().ok_or_else(|| {
        format!("GTEx v2 histology API returned no records for '{api_url}'")
    })?;

    if sample.subject_id.trim().is_empty() || sample.histology_image_id.trim().is_empty() {
        return Err(format!(
            "GTEx v2 histology API returned an invalid record: subject_id='{}' histology_image_id='{}'",
            sample.subject_id, sample.histology_image_id
        )
        .into());
    }

    let dzi_url = format!(
        "{GTEX_HISTOLOGY_DZI_URL_ROOT}{}/{}.dzi",
        sample.subject_id, sample.histology_image_id
    );

    let dzi_response = client.get(&dzi_url).send().await?;
    let dzi_status = dzi_response.status();
    let dzi_xml = dzi_response.text().await.unwrap_or_default();
    if !dzi_status.is_success() {
        return Err(format!(
            "GTEx DeepZoom descriptor request failed: GET '{dzi_url}' returned status {dzi_status}; body: {dzi_xml}"
        )
        .into());
    }

    let format = extract_dzi_format(&dzi_xml)?;
    let tile_url = format!(
        "{GTEX_HISTOLOGY_DZI_URL_ROOT}{}/{}_files/0/0_0.{format}",
        sample.subject_id, sample.histology_image_id
    );

    Ok(GtexResolvedFov {
        subject_id: sample.subject_id,
        histology_image_id: sample.histology_image_id,
        tissue_site_detail: sample.tissue_site_detail,
        dzi_url,
        tile_url,
    })
}

async fn validate_gtex_fov_url(
    client: &Client,
    cohort_label: &str,
    example: &GtexResolvedFov,
) -> Result<(), AnyError> {
    let url = example.tile_url.as_str();

    let body = fetch_range_bytes(client, url, "bytes=0-1023")
        .await
        .map_err(|error| {
            format!(
                "GTEx FOV range validation failed for URL '{}' (cohort '{}', subject_id='{}', histology_image_id='{}', tissue='{}'): {error}",
                url,
                cohort_label,
                example.subject_id,
                example.histology_image_id,
                example.tissue_site_detail
            )
        })?;
    assert!(
        !body.is_empty(),
        "Expected GTEx FOV URL '{}' (cohort '{}', subject_id='{}', histology_image_id='{}', tissue='{}') to return non-empty body for range GET",
        url,
        cohort_label,
        example.subject_id,
        example.histology_image_id,
        example.tissue_site_detail,
    );

    let response = fetch_head_or_range(client, url).await.map_err(|error| {
        format!(
            "GTEx FOV HEAD/range probe failed for URL '{}' (cohort '{}', subject_id='{}', histology_image_id='{}', tissue='{}'): {error}",
            url,
            cohort_label,
            example.subject_id,
            example.histology_image_id,
            example.tissue_site_detail
        )
    })?;

    if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
        let ct = content_type
            .to_str()
            .unwrap_or_default()
            .to_ascii_lowercase();
        assert!(
            ct.starts_with("image/")
                || ct.contains("tiff")
                || ct.contains("jpeg")
                || ct.contains("png")
                || ct.contains("octet-stream"),
            "Expected GTEx FOV URL '{}' (cohort '{}', subject_id='{}', histology_image_id='{}', tissue='{}') to return an image-like content-type, got '{}'",
            url,
            cohort_label,
            example.subject_id,
            example.histology_image_id,
            example.tissue_site_detail,
            ct
        );
    }

    Ok(())
}

#[tokio::test]
async fn gtex_fov_resolves_via_v2_api() -> Result<(), AnyError> {
    let client = gtex_client()?;
    let example = resolve_gtex_fov_from_v2_api(&client).await?;

    assert!(
        example.dzi_url.starts_with("http://") || example.dzi_url.starts_with("https://"),
        "Resolved GTEx DZI URL must be HTTP(S), got '{}'",
        example.dzi_url
    );
    assert!(
        example.tile_url.starts_with("http://") || example.tile_url.starts_with("https://"),
        "Resolved GTEx tile URL must be HTTP(S), got '{}'",
        example.tile_url
    );

    Ok(())
}

#[tokio::test]
async fn gtex_fov_tile_range_request_valid() -> Result<(), AnyError> {
    let client = gtex_client()?;
    let example = resolve_gtex_fov_from_v2_api(&client).await?;
    validate_gtex_fov_url(&client, "GTEx-Histology-v2", &example).await
}
