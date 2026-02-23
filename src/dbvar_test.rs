use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::time::Duration;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const DBVAR_API_BASE: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";
const BRCA2_DBVAR_ACC: &str = "nsv7897110";

fn dbvar_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (dbvar-integration-test)")
        .timeout(Duration::from_secs(25))
        .build()?;
    Ok(client)
}

fn dbvar_variant_url(accession: &str) -> String {
    format!("{DBVAR_API_BASE}/esearch.fcgi?db=dbvar&term=%22{accession}%22&retmode=json")
}

fn dbvar_summary_url(uid: &str) -> String {
    format!("{DBVAR_API_BASE}/esummary.fcgi?db=dbvar&id={uid}&retmode=json")
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
            "dbVar request failed for URL '{url}' with status {status}: {snippet}"
        )
        .into());
    }

    serde_json::from_str::<T>(&text).map_err(|error| {
        format!(
            "Failed to deserialize dbVar JSON response from URL '{url}': {error}; body: {snippet}"
        )
        .into()
    })
}

#[derive(Debug, Deserialize)]
struct DbvarSearchEnvelope {
    esearchresult: DbvarSearchResult,
}

#[derive(Debug, Deserialize)]
struct DbvarSearchResult {
    #[serde(default)]
    idlist: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct DbvarSummaryEnvelope {
    result: DbvarSummaryResult,
}

// ESummary response shape: a `uids` list plus per-UID records keyed by UID string.
#[derive(Debug, Deserialize)]
struct DbvarSummaryResult {
    #[serde(default)]
    uids: Vec<String>,
    #[serde(flatten)]
    records: HashMap<String, DbvarVariant>,
}

#[derive(Debug, Deserialize, Clone)]
struct DbvarVariant {
    #[serde(default)]
    sv: String,
    #[serde(default)]
    dbvarvarianttypelist: Vec<String>,
    #[serde(default)]
    dbvarplacementlist: Vec<DbvarPlacement>,
    #[serde(default)]
    dbvargenelist: Vec<DbvarGene>,
}

#[derive(Debug, Deserialize, Clone)]
struct DbvarPlacement {
    #[serde(default)]
    chr: String,
    #[serde(default)]
    chr_start: i64,
    #[serde(default)]
    chr_end: i64,
}

#[derive(Debug, Deserialize, Clone)]
struct DbvarGene {
    #[serde(default)]
    name: String,
    #[serde(default)]
    id: i64,
}

async fn fetch_dbvar_variant_by_accession(
    client: &Client,
    accession: &str,
) -> Result<DbvarVariant, AnyError> {
    // dbVar retrieval is a two-step flow: ESearch accession -> UID, then ESummary UID -> record.
    let search_url = dbvar_variant_url(accession);
    let search: DbvarSearchEnvelope = fetch_json(client, &search_url).await?;

    let uid = search
        .esearchresult
        .idlist
        .first()
        .cloned()
        .ok_or_else(|| {
            format!(
                "dbVar search returned no records for accession '{}' using URL '{}'",
                accession, search_url
            )
        })?;

    let summary_url = dbvar_summary_url(&uid);
    let summary: DbvarSummaryEnvelope = fetch_json(client, &summary_url).await?;

    if !summary.result.uids.iter().any(|id| id == &uid) {
        return Err(format!(
            "dbVar summary response for accession '{}' did not include requested uid '{}' in uids list from URL '{}'",
            accession, uid, summary_url
        )
        .into());
    }

    summary.result.records.get(&uid).cloned().ok_or_else(|| {
        format!(
            "dbVar summary response for accession '{}' missing record object for uid '{}' from URL '{}'",
            accession, uid, summary_url
        )
        .into()
    })
}

#[tokio::test]
async fn dbvar_brca2_sv_basic_fields() -> Result<(), AnyError> {
    let client = dbvar_client()?;
    let variant = fetch_dbvar_variant_by_accession(&client, BRCA2_DBVAR_ACC).await?;

    assert!(
        variant.sv == BRCA2_DBVAR_ACC || variant.sv.ends_with(BRCA2_DBVAR_ACC),
        "Expected dbVar variant accession '{}' in payload, got '{}'",
        BRCA2_DBVAR_ACC,
        variant.sv
    );

    assert!(
        !variant.dbvarvarianttypelist.is_empty()
            && variant
                .dbvarvarianttypelist
                .iter()
                .any(|variant_type| !variant_type.trim().is_empty()),
        "Expected dbVar variant '{}' to have a non-empty structural variant type list, got {:?}",
        BRCA2_DBVAR_ACC,
        variant.dbvarvarianttypelist
    );

    assert!(
        !variant.dbvarplacementlist.is_empty(),
        "Expected dbVar variant '{}' to include at least one placement / coordinate block",
        BRCA2_DBVAR_ACC
    );

    let mut has_chr13_span = false;
    for placement in &variant.dbvarplacementlist {
        if placement.chr == "13"
            || placement.chr.contains("NC_000013")
            || placement.chr.eq_ignore_ascii_case("chr13")
        {
            assert!(
                placement.chr_start >= 0 && placement.chr_end >= placement.chr_start,
                "Expected dbVar variant '{}' placement on chr13 to have start >= 0 and end >= start, got start={}, end={}",
                BRCA2_DBVAR_ACC,
                placement.chr_start,
                placement.chr_end
            );
            has_chr13_span = true;
        }
    }
    assert!(
        has_chr13_span,
        "Expected dbVar variant '{}' to include at least one placement on chr13",
        BRCA2_DBVAR_ACC
    );

    let has_brca2_gene = variant.dbvargenelist.iter().any(|gene| {
        gene.name.eq_ignore_ascii_case("BRCA2")
            || gene.name.to_ascii_uppercase().contains("BRCA2")
            || gene.id == 675
    });

    assert!(
        has_brca2_gene,
        "Expected dbVar variant '{}' to be associated with BRCA2 (symbol contains 'BRCA2' or gene id 675)",
        BRCA2_DBVAR_ACC
    );

    Ok(())
}

#[tokio::test]
async fn dbvar_invalid_accession_returns_error() -> Result<(), AnyError> {
    let client = dbvar_client()?;
    let bad_acc = "nsvNOT_A_REAL_ID_123456";

    let result = fetch_dbvar_variant_by_accession(&client, bad_acc).await;
    assert!(
        result.is_err(),
        "Expected dbVar variant fetch to fail for invalid accession '{}' at URL '{}'",
        bad_acc,
        dbvar_variant_url(bad_acc)
    );

    if let Err(error) = result {
        let msg = error.to_string();
        assert!(
            msg.contains("NOT_A_REAL_ID_123456")
                || msg.contains("404")
                || msg.contains("500")
                || msg.to_ascii_lowercase().contains("not found")
                || msg.to_ascii_lowercase().contains("no records")
                || msg.to_ascii_lowercase().contains("cannot"),
            "Expected dbVar error message for invalid accession '{}' to mention the invalid ID or a not-found/no-records style failure, got '{}'",
            bad_acc,
            msg
        );
    }

    Ok(())
}
