use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use std::time::Duration;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const DBSNP_API_BASE: &str = "https://api.ncbi.nlm.nih.gov/variation/v0";
const BRCA2_RSID: &str = "rs80359350";

fn dbsnp_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (dbsnp-integration-test)")
        .timeout(Duration::from_secs(25))
        .build()?;
    Ok(client)
}

fn dbsnp_variant_url(rsid: &str) -> String {
    let numeric_id = rsid.strip_prefix("rs").unwrap_or(rsid);
    format!("{DBSNP_API_BASE}/refsnp/{numeric_id}")
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
        return Err(
            format!("dbSNP request failed for URL '{url}' with status {status}: {snippet}").into(),
        );
    }

    serde_json::from_str::<T>(&text).map_err(|error| {
        format!(
            "Failed to deserialize dbSNP JSON response from URL '{url}': {error}; body: {snippet}"
        )
        .into()
    })
}

#[derive(Debug, Deserialize)]
struct DbsnpVariant {
    refsnp_id: String,
    primary_snapshot_data: DbsnpPrimarySnapshot,
}

#[derive(Debug, Deserialize)]
struct DbsnpPrimarySnapshot {
    #[serde(default)]
    placements_with_allele: Vec<DbsnpPlacement>,
    #[serde(default)]
    allele_annotations: Vec<DbsnpAlleleAnnotation>,
}

#[derive(Debug, Deserialize)]
struct DbsnpPlacement {
    seq_id: String,
    is_ptlp: bool,
    #[serde(default)]
    alleles: Vec<DbsnpPlacedAllele>,
}

#[derive(Debug, Deserialize)]
struct DbsnpPlacedAllele {
    #[serde(default)]
    allele: Option<DbsnpAllele>,
}

#[derive(Debug, Deserialize)]
struct DbsnpAllele {
    #[serde(default)]
    spdi: Option<DbsnpSpdi>,
}

#[derive(Debug, Deserialize)]
struct DbsnpSpdi {
    seq_id: String,
    position: i64,
    deleted_sequence: String,
    inserted_sequence: String,
}

#[derive(Debug, Deserialize)]
struct DbsnpAlleleAnnotation {
    #[serde(default)]
    assembly_annotation: Vec<DbsnpAssemblyAnnotation>,
}

#[derive(Debug, Deserialize)]
struct DbsnpAssemblyAnnotation {
    #[serde(default)]
    genes: Vec<DbsnpGene>,
}

#[derive(Debug, Deserialize)]
struct DbsnpGene {
    #[serde(default)]
    locus: String,
    #[serde(default)]
    id: i64,
}

fn is_dna_like(seq: &str) -> bool {
    !seq.is_empty()
        && seq
            .chars()
            .all(|base| matches!(base.to_ascii_uppercase(), 'A' | 'C' | 'G' | 'T' | 'N'))
}

#[tokio::test]
async fn dbsnp_brca2_variant_basic_fields() -> Result<(), AnyError> {
    let client = dbsnp_client()?;
    let url = dbsnp_variant_url(BRCA2_RSID);
    let variant: DbsnpVariant = fetch_json(&client, &url).await?;

    assert_eq!(
        variant.refsnp_id,
        BRCA2_RSID.trim_start_matches("rs"),
        "Expected dbSNP refsnp_id '{}' for requested rsID '{}', got '{}'",
        BRCA2_RSID.trim_start_matches("rs"),
        BRCA2_RSID,
        variant.refsnp_id
    );

    let placements = &variant.primary_snapshot_data.placements_with_allele;
    assert!(
        !placements.is_empty(),
        "Expected dbSNP variant '{}' to include at least one placement_with_allele entry",
        BRCA2_RSID
    );

    let top_level = placements
        .iter()
        .find(|placement| placement.is_ptlp)
        .ok_or_else(|| {
            format!(
                "Expected dbSNP variant '{}' to include at least one top-level placement (is_ptlp=true); got {} placements",
                BRCA2_RSID,
                placements.len()
            )
        })?;

    assert!(
        top_level.seq_id.contains("NC_000013"),
        "Expected dbSNP variant '{}' top-level seq_id to map to chromosome 13, got '{}'",
        BRCA2_RSID,
        top_level.seq_id
    );

    let mut has_position = false;
    let mut has_sane_allele = false;

    for placed in &top_level.alleles {
        if let Some(spdi) = placed.allele.as_ref().and_then(|allele| allele.spdi.as_ref()) {
            if spdi.position >= 0 {
                has_position = true;
            }

            let deleted = spdi.deleted_sequence.trim();
            let inserted = spdi.inserted_sequence.trim();
            let valid_deleted = is_dna_like(deleted) && deleted.len() < 100;
            let valid_inserted = is_dna_like(inserted) && inserted.len() < 100;

            if (valid_deleted || valid_inserted) && spdi.seq_id.contains("NC_000013") {
                has_sane_allele = true;
            }
        }
    }

    assert!(
        has_position,
        "Expected dbSNP variant '{}' to include at least one non-negative genomic position in top-level alleles",
        BRCA2_RSID
    );
    assert!(
        has_sane_allele,
        "Expected dbSNP variant '{}' to include at least one DNA-like ref/alt allele with length < 100bp in top-level placement",
        BRCA2_RSID
    );

    let has_brca2_gene = variant
        .primary_snapshot_data
        .allele_annotations
        .iter()
        .flat_map(|annotation| annotation.assembly_annotation.iter())
        .flat_map(|assembly| assembly.genes.iter())
        .any(|gene| {
            gene.locus.eq_ignore_ascii_case("BRCA2")
                || gene.locus.to_ascii_uppercase().contains("BRCA2")
                || gene.id == 675
        });

    assert!(
        has_brca2_gene,
        "Expected dbSNP variant '{}' to include BRCA2 gene association (locus contains 'BRCA2' or gene id 675)",
        BRCA2_RSID
    );

    Ok(())
}

#[tokio::test]
async fn dbsnp_invalid_rsid_returns_error() -> Result<(), AnyError> {
    let client = dbsnp_client()?;
    let bad_rsid = "rsNOT_A_REAL_ID_123456";
    let url = dbsnp_variant_url(bad_rsid);

    let result: Result<DbsnpVariant, AnyError> = fetch_json(&client, &url).await;
    assert!(
        result.is_err(),
        "Expected dbSNP fetch_json to fail for invalid rsID '{}' at URL '{}'",
        bad_rsid,
        url
    );

    if let Err(error) = result {
        let msg = error.to_string();
        assert!(
            msg.contains("NOT_A_REAL_ID_123456")
                || msg.contains("404")
                || msg.contains("500")
                || msg.to_ascii_lowercase().contains("not found")
                || msg.to_ascii_lowercase().contains("no records")
                || msg.to_ascii_lowercase().contains("cannot convert"),
            "Expected dbSNP error message for invalid rsID '{}' to mention the invalid numeric ID or a no-records/not-found style failure, got '{}'",
            bad_rsid,
            msg
        );
    }

    Ok(())
}
