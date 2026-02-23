use reqwest::Client;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::json;
use std::error::Error;

const GNOMAD_GRAPHQL_URL: &str = "https://gnomad.broadinstitute.org/api";
const BRCA2_VARIANT_ID: &str = "13-32316386-C-T";

fn is_dna_base(base: char) -> bool {
    matches!(base.to_ascii_uppercase(), 'A' | 'C' | 'G' | 'T')
}

fn gnomad_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (gnomad-integration-test)")
        .build()
}

#[derive(Debug, Deserialize)]
struct GraphqlError {
    message: String,
}

#[derive(Debug, Deserialize)]
struct GraphqlEnvelope<T> {
    data: Option<T>,
    #[serde(default)]
    errors: Vec<GraphqlError>,
}

async fn gnomad_graphql_query<T: DeserializeOwned>(
    client: &Client,
    query: &str,
    variables: serde_json::Value,
) -> Result<T, Box<dyn Error>> {
    let body = json!({
        "query": query,
        "variables": variables,
    });

    let response = client.post(GNOMAD_GRAPHQL_URL).json(&body).send().await?;
    let status = response.status();

    if !status.is_success() {
        let text = response.text().await.unwrap_or_default();
        return Err(format!(
            "gnomAD GraphQL request failed for URL '{}' with status {}: {}",
            GNOMAD_GRAPHQL_URL, status, text
        )
        .into());
    }

    let envelope: GraphqlEnvelope<T> = response.json().await?;
    if !envelope.errors.is_empty() {
        let combined = envelope
            .errors
            .iter()
            .map(|error| error.message.as_str())
            .collect::<Vec<_>>()
            .join("; ");
        return Err(format!("gnomAD GraphQL returned errors: {combined}").into());
    }

    envelope
        .data
        .ok_or_else(|| {
            format!(
                "gnomAD GraphQL response contained no data payload for query: {query}"
            )
            .into()
        })
}

#[derive(Debug, Deserialize)]
struct GnomadVariantQueryData {
    variant: Option<GnomadVariantRecord>,
}

#[derive(Debug, Deserialize)]
struct GnomadVariantRecord {
    #[serde(rename = "variant_id")]
    variant_id: String,
    chrom: String,
    pos: u64,
    #[serde(rename = "ref")]
    ref_allele: String,
    alt: String,
    exome: Option<GnomadVariantDatasetStats>,
    genome: Option<GnomadVariantDatasetStats>,
}

#[derive(Debug, Deserialize)]
struct GnomadVariantDatasetStats {
    ac: i64,
    an: i64,
    af: f64,
    #[serde(default)]
    populations: Vec<GnomadVariantPopulationRaw>,
}

#[derive(Debug, Deserialize)]
struct GnomadVariantPopulationRaw {
    id: String,
    ac: i64,
    an: i64,
}

#[derive(Debug)]
struct GnomadVariantCore {
    variant_id: String,
    chrom: String,
    pos: u64,
    ref_allele: String,
    alt_allele: String,
    dataset: String,
    ac: i64,
    an: i64,
    af: f64,
}

#[derive(Debug)]
struct GnomadPopulationFrequency {
    id: String,
    dataset: String,
    ac: i64,
    an: i64,
    af: f64,
}

const GNOMAD_VARIANT_QUERY: &str = r#"
query($variantId:String!) {
  variant(variantId:$variantId, dataset:gnomad_r4) {
    variant_id
    chrom
    pos
    ref
    alt
    exome {
      ac
      an
      af
      populations {
        id
        ac
        an
      }
    }
    genome {
      ac
      an
      af
      populations {
        id
        ac
        an
      }
    }
  }
}
"#;

async fn gnomad_fetch_variant(
    client: &Client,
    variant_id: &str,
) -> Result<GnomadVariantCore, Box<dyn Error>> {
    let data: GnomadVariantQueryData = gnomad_graphql_query(
        client,
        GNOMAD_VARIANT_QUERY,
        json!({ "variantId": variant_id }),
    )
    .await
    .map_err(|error| {
        format!(
            "Failed to fetch gnomAD variant core fields for variant '{}': {}",
            variant_id, error
        )
    })?;

    let variant = data.variant.ok_or_else(|| {
        format!(
            "gnomAD returned null variant payload for variant id '{}'",
            variant_id
        )
    })?;

    let selected = if let Some(exome) = &variant.exome {
        if exome.an > 0 {
            Some(("exome", exome))
        } else {
            None
        }
    } else {
        None
    }
    .or_else(|| {
        variant
            .genome
            .as_ref()
            .filter(|genome| genome.an > 0)
            .map(|genome| ("genome", genome))
    })
    .ok_or_else(|| {
        format!(
            "gnomAD variant '{}' had no dataset with AN > 0 in exome or genome",
            variant_id
        )
    })?;

    assert!(
        selected.1.ac >= 0,
        "Expected non-negative AC for variant '{}' in {} dataset, got {}",
        variant_id,
        selected.0,
        selected.1.ac
    );
    assert!(
        selected.1.an >= 0,
        "Expected non-negative AN for variant '{}' in {} dataset, got {}",
        variant_id,
        selected.0,
        selected.1.an
    );
    assert!(
        selected.1.af >= 0.0 && selected.1.af <= 1.0,
        "Expected AF in [0,1] for variant '{}' in {} dataset, got {}",
        variant_id,
        selected.0,
        selected.1.af
    );

    Ok(GnomadVariantCore {
        variant_id: variant.variant_id,
        chrom: variant.chrom,
        pos: variant.pos,
        ref_allele: variant.ref_allele,
        alt_allele: variant.alt,
        dataset: selected.0.to_string(),
        ac: selected.1.ac,
        an: selected.1.an,
        af: selected.1.af,
    })
}

async fn gnomad_fetch_variant_populations(
    client: &Client,
    variant_id: &str,
) -> Result<Vec<GnomadPopulationFrequency>, Box<dyn Error>> {
    let data: GnomadVariantQueryData = gnomad_graphql_query(
        client,
        GNOMAD_VARIANT_QUERY,
        json!({ "variantId": variant_id }),
    )
    .await
    .map_err(|error| {
        format!(
            "Failed to fetch gnomAD population frequencies for variant '{}': {}",
            variant_id, error
        )
    })?;

    let variant = data.variant.ok_or_else(|| {
        format!(
            "gnomAD returned null variant payload for variant id '{}'",
            variant_id
        )
    })?;

    let mut populations = Vec::new();

    if let Some(exome) = variant.exome {
        for population in exome.populations {
            let af = if population.an > 0 {
                population.ac as f64 / population.an as f64
            } else {
                0.0
            };
            populations.push(GnomadPopulationFrequency {
                id: population.id,
                dataset: "exome".to_string(),
                ac: population.ac,
                an: population.an,
                af,
            });
        }
    }

    if let Some(genome) = variant.genome {
        for population in genome.populations {
            let af = if population.an > 0 {
                population.ac as f64 / population.an as f64
            } else {
                0.0
            };
            populations.push(GnomadPopulationFrequency {
                id: population.id,
                dataset: "genome".to_string(),
                ac: population.ac,
                an: population.an,
                af,
            });
        }
    }

    if populations.is_empty() {
        // This integration-style example uses a well-known BRCA2 variant where
        // population data is expected to be present.
        return Err(format!(
            "gnomAD returned no population frequencies for variant '{}'",
            variant_id
        )
        .into());
    }

    Ok(populations)
}

#[tokio::test]
async fn fetch_gnomad_brca2_variant_core_fields() -> Result<(), Box<dyn Error>> {
    let client = gnomad_client()?;
    let variant = gnomad_fetch_variant(&client, BRCA2_VARIANT_ID).await?;

    assert_eq!(
        variant.variant_id, BRCA2_VARIANT_ID,
        "Expected gnomAD variant_id '{}' for BRCA2 example, got '{}'",
        BRCA2_VARIANT_ID, variant.variant_id
    );
    assert_eq!(
        variant.chrom, "13",
        "Expected gnomAD chrom '13' for BRCA2 variant '{}', got '{}'",
        BRCA2_VARIANT_ID, variant.chrom
    );
    assert!(
        (32_300_000..=32_500_000).contains(&variant.pos),
        "Expected BRCA2-region position on chr13 for '{}', got {}",
        BRCA2_VARIANT_ID,
        variant.pos
    );
    assert!(
        variant.ref_allele.len() == 1 && variant.alt_allele.len() == 1,
        "Expected SNV alleles of length 1 for '{}', got ref='{}', alt='{}'",
        BRCA2_VARIANT_ID,
        variant.ref_allele,
        variant.alt_allele
    );
    let ref_base = variant.ref_allele.chars().next().unwrap_or('?');
    let alt_base = variant.alt_allele.chars().next().unwrap_or('?');
    assert!(
        is_dna_base(ref_base) && is_dna_base(alt_base),
        "Expected SNV alleles to be valid DNA bases (A/C/G/T) for '{}', got ref='{}', alt='{}'",
        BRCA2_VARIANT_ID,
        variant.ref_allele,
        variant.alt_allele
    );
    assert!(
        variant.af >= 0.0 && variant.af <= 1.0,
        "Expected AF in [0,1] for '{}', got {} from {} dataset",
        BRCA2_VARIANT_ID,
        variant.af,
        variant.dataset
    );
    assert!(
        variant.af > 0.0 && variant.af < 1.0,
        "Expected AF in (0,1) for '{}', got {} from {} dataset",
        BRCA2_VARIANT_ID,
        variant.af,
        variant.dataset
    );
    assert!(
        variant.ac > 0,
        "Expected AC > 0 for '{}', got {} from {} dataset",
        BRCA2_VARIANT_ID,
        variant.ac,
        variant.dataset
    );
    assert!(
        variant.an > 0,
        "Expected AN > 0 for '{}', got {} from {} dataset",
        BRCA2_VARIANT_ID,
        variant.an,
        variant.dataset
    );
    assert!(
        variant.ac <= variant.an,
        "Expected AC <= AN for '{}', got AC={}, AN={} from {} dataset",
        BRCA2_VARIANT_ID,
        variant.ac,
        variant.an,
        variant.dataset
    );

    Ok(())
}

#[tokio::test]
async fn fetch_gnomad_brca2_variant_population_frequencies() -> Result<(), Box<dyn Error>> {
    let client = gnomad_client()?;
    let populations = gnomad_fetch_variant_populations(&client, BRCA2_VARIANT_ID).await?;

    assert!(
        !populations.is_empty(),
        "Expected non-empty gnomAD population frequencies for '{}'",
        BRCA2_VARIANT_ID
    );

    let mut has_non_zero_af = false;
    for population in &populations {
        assert!(
            !population.id.trim().is_empty(),
            "Expected non-empty population id in gnomAD population frequencies"
        );
        assert!(
            !population.dataset.trim().is_empty(),
            "Expected dataset tag (exome/genome) in gnomAD population frequencies"
        );
        assert!(
            population.af >= 0.0 && population.af <= 1.0,
            "Expected AF in [0,1] for population '{}' ({}), got {}",
            population.id,
            population.dataset,
            population.af
        );
        assert!(
            population.ac >= 0,
            "Expected AC >= 0 for population '{}' ({}), got {}",
            population.id,
            population.dataset,
            population.ac
        );
        assert!(
            population.an >= 0,
            "Expected AN >= 0 for population '{}' ({}), got {}",
            population.id,
            population.dataset,
            population.an
        );
        if population.an > 0 {
            assert!(
                population.ac <= population.an,
                "Expected AC <= AN for population '{}' ({}), got AC={}, AN={}",
                population.id,
                population.dataset,
                population.ac,
                population.an
            );
        }

        if population.af > 0.0 {
            has_non_zero_af = true;
        }
    }

    assert!(
        has_non_zero_af,
        "Expected at least one population with non-zero AF for '{}'",
        BRCA2_VARIANT_ID
    );

    Ok(())
}

#[tokio::test]
async fn gnomad_invalid_variant_returns_error() {
    let client = gnomad_client().unwrap();
    let bad = "NOT_A_REAL_VARIANT";

    let core_result = gnomad_fetch_variant(&client, bad).await;
    assert!(
        core_result.is_err(),
        "Expected gnomad_fetch_variant to return error for invalid variant id '{}'",
        bad
    );
    if let Err(error) = &core_result {
        let message = error.to_string();
        assert!(
            message.contains(bad),
            "Expected core error message to include invalid variant id '{}', got '{}'",
            bad,
            message
        );
    }

    let population_result = gnomad_fetch_variant_populations(&client, bad).await;
    assert!(
        population_result.is_err(),
        "Expected gnomad_fetch_variant_populations to return error for invalid variant id '{}'",
        bad
    );
    if let Err(error) = &population_result {
        let message = error.to_string();
        assert!(
            message.contains(bad),
            "Expected population error message to include invalid variant id '{}', got '{}'",
            bad,
            message
        );
    }
}
