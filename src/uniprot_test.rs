use reqwest::Client;
use serde::Deserialize;
use std::error::Error;

fn uniprot_base_url() -> &'static str {
    "https://rest.uniprot.org"
}

fn uniprot_protein_url(accession: &str) -> String {
    format!("{}/uniprotkb/{}.json", uniprot_base_url(), accession)
}

fn uniprot_fasta_url(accession: &str) -> String {
    format!("{}/uniprotkb/{}.fasta", uniprot_base_url(), accession)
}

fn uniprot_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (uniprot-integration-test)")
        .build()
}

#[derive(Debug, Deserialize)]
struct UniProtEntry {
    #[serde(rename = "primaryAccession")]
    primary_accession: String,
    #[serde(default)]
    organism: UniProtOrganism,
    #[serde(default, rename = "proteinDescription")]
    protein_description: UniProtProteinDescription,
    #[serde(default, rename = "uniProtKBCrossReferences")]
    uni_prot_kb_cross_references: Vec<UniProtXref>,
}

#[derive(Debug, Default, Deserialize)]
struct UniProtOrganism {
    #[serde(default, rename = "scientificName")]
    scientific_name: String,
    #[serde(default, rename = "commonName")]
    common_name: String,
    #[serde(default, rename = "taxonId")]
    taxonomy_id: Option<u64>,
    #[serde(default, rename = "taxonomy")]
    taxonomy: Vec<u64>,
}

#[derive(Debug, Default, Deserialize)]
struct UniProtProteinDescription {
    #[serde(default, rename = "recommendedName")]
    recommended_name: Option<UniProtRecommendedName>,
}

#[derive(Debug, Default, Deserialize)]
struct UniProtRecommendedName {
    #[serde(default, rename = "fullName")]
    full_name: Option<UniProtTextValue>,
}

#[derive(Debug, Default, Deserialize)]
struct UniProtTextValue {
    #[serde(default)]
    value: String,
}

#[derive(Debug, Default, Deserialize)]
struct UniProtXref {
    #[serde(default)]
    database: String,
    #[serde(default)]
    id: String,
}

async fn uniprot_fetch_entry(
    client: &Client,
    accession: &str,
) -> Result<UniProtEntry, Box<dyn Error>> {
    let url = uniprot_protein_url(accession);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "UniProt JSON request failed for accession '{accession}' with status {status}: {body}"
        )
        .into());
    }
    let entry: UniProtEntry = response.json().await?;
    Ok(entry)
}

async fn uniprot_fetch_fasta_sequence(
    client: &Client,
    accession: &str,
) -> Result<String, Box<dyn Error>> {
    let url = uniprot_fasta_url(accession);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "UniProt FASTA request failed for accession '{accession}' with status {status}: {body}"
        )
        .into());
    }
    let text = response.text().await?;

    assert!(
        text.lines().any(|line| line.starts_with('>')),
        "Expected UniProt FASTA for '{}' to contain a header line starting with '>'",
        accession
    );

    let mut seq = String::new();
    for line in text.lines() {
        if line.starts_with('>') {
            continue;
        }
        seq.push_str(line.trim());
    }

    Ok(seq)
}

fn is_protein_sequence(seq: &str) -> bool {
    seq.chars().all(|residue| {
        matches!(
            residue.to_ascii_uppercase(),
            'A' | 'C'
                | 'D'
                | 'E'
                | 'F'
                | 'G'
                | 'H'
                | 'I'
                | 'K'
                | 'L'
                | 'M'
                | 'N'
                | 'P'
                | 'Q'
                | 'R'
                | 'S'
                | 'T'
                | 'V'
                | 'W'
                | 'Y'
                | 'B'
                | 'Z'
                | 'X'
                | 'U'
                | 'O'
                | '*'
        )
    })
}

#[tokio::test]
async fn fetch_uniprot_brca2_entry() -> Result<(), Box<dyn Error>> {
    let client = uniprot_client()?;
    let accession = "P51587";

    let entry = uniprot_fetch_entry(&client, accession).await?;

    assert_eq!(
        entry.primary_accession, accession,
        "Expected UniProt primaryAccession '{}' for BRCA2, got '{}'",
        accession, entry.primary_accession
    );

    assert!(
        entry
            .organism
            .scientific_name
            .eq_ignore_ascii_case("Homo sapiens"),
        "Expected organism scientificName 'Homo sapiens', got '{}'",
        entry.organism.scientific_name
    );
    let has_human_taxon =
        entry.organism.taxonomy_id == Some(9606) || entry.organism.taxonomy.contains(&9606);
    assert!(
        has_human_taxon,
        "Expected UniProt organism taxonomy to include taxon id 9606, got taxonId={:?}, taxonomy={:?}",
        entry.organism.taxonomy_id, entry.organism.taxonomy
    );

    if !entry.organism.common_name.trim().is_empty() {
        assert!(
            entry.organism.common_name.eq_ignore_ascii_case("Human"),
            "Expected commonName to be 'Human' when present, got '{}'",
            entry.organism.common_name
        );
    }

    if let Some(recommended) = entry.protein_description.recommended_name.as_ref()
        && let Some(full_name) = recommended.full_name.as_ref()
    {
        let name = full_name.value.to_ascii_lowercase();
        assert!(
            name.contains("brca2") || name.contains("breast cancer type 2"),
            "Expected recommended fullName to mention BRCA2 / breast cancer type 2, got '{}'",
            full_name.value
        );
    }

    Ok(())
}

#[tokio::test]
async fn fetch_uniprot_brca2_sequence() -> Result<(), Box<dyn Error>> {
    let client = uniprot_client()?;
    let accession = "P51587";

    let seq = uniprot_fetch_fasta_sequence(&client, accession).await?;
    let trimmed = seq.trim();

    assert!(
        !trimmed.is_empty(),
        "Expected non-empty UniProt sequence for accession '{}'",
        accession
    );
    assert!(
        is_protein_sequence(trimmed),
        "Expected valid amino-acid alphabet for UniProt sequence '{}', got leading snippet '{}'",
        accession,
        trimmed.chars().take(40).collect::<String>()
    );
    assert!(
        trimmed.len() > 1000,
        "BRCA2 is a large protein; expected length > 1000 aa, got {}",
        trimmed.len()
    );

    Ok(())
}

#[tokio::test]
async fn fetch_uniprot_brca2_xrefs() -> Result<(), Box<dyn Error>> {
    let client = uniprot_client()?;
    let accession = "P51587";

    let entry = uniprot_fetch_entry(&client, accession).await?;

    assert!(
        !entry.uni_prot_kb_cross_references.is_empty(),
        "Expected UniProt BRCA2 entry to have at least one cross-reference"
    );

    let mut has_ensembl = false;
    let mut has_pdb = false;

    for xref in &entry.uni_prot_kb_cross_references {
        let db = xref.database.to_ascii_uppercase();
        if db == "ENSEMBL" {
            has_ensembl = true;
            assert!(
                !xref.id.trim().is_empty(),
                "Expected ENSEMBL cross-reference id to be non-empty"
            );
            let base_id = xref
                .id
                .split_once('.')
                .map(|(prefix, _)| prefix)
                .unwrap_or(xref.id.as_str());
            assert!(
                base_id.starts_with("ENSG")
                    || base_id.starts_with("ENST")
                    || base_id.starts_with("ENSP"),
                "Expected ENSEMBL cross-reference id to start with ENSG/ENST/ENSP, got '{}'",
                xref.id
            );

            if xref.id == "ENSP00000419060" || xref.id == "ENST00000380152" {
                assert!(
                    xref.id.starts_with("ENSP") || xref.id.starts_with("ENST"),
                    "Expected known Ensembl BRCA2 cross-reference to be well-formed, got '{}'",
                    xref.id
                );
            }
        } else if db == "PDB" {
            has_pdb = true;
            assert!(
                !xref.id.trim().is_empty(),
                "Expected PDB cross-reference id to be non-empty"
            );
            assert_eq!(
                xref.id.len(),
                4,
                "Expected PDB cross-reference id to be 4 characters, got '{}'",
                xref.id
            );
        }
    }

    assert!(
        has_ensembl,
        "Expected UniProt BRCA2 entry to have at least one ENSEMBL cross-reference"
    );
    assert!(
        has_pdb,
        "Expected UniProt BRCA2 entry to have at least one PDB cross-reference"
    );

    Ok(())
}

#[tokio::test]
async fn uniprot_invalid_accession_returns_error() {
    let client = uniprot_client().unwrap();
    let bad = "NOT_A_REAL_ACCESSION";

    let json_result = uniprot_fetch_entry(&client, bad).await;
    assert!(
        json_result.is_err(),
        "Expected UniProt JSON fetch for invalid accession '{}' to return an error",
        bad
    );

    let fasta_result = uniprot_fetch_fasta_sequence(&client, bad).await;
    assert!(
        fasta_result.is_err(),
        "Expected UniProt FASTA fetch for invalid accession '{}' to return an error",
        bad
    );
}
