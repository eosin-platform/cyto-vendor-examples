use reqwest::Client;
use serde::Deserialize;
use std::error::Error;

fn ensembl_base_url() -> &'static str {
    "https://rest.ensembl.org"
}

fn ensembl_lookup_id_url(id: &str) -> String {
    format!(
        "{}/lookup/id/{}?content-type=application/json",
        ensembl_base_url(),
        id
    )
}

fn ensembl_variation_url(species: &str, id: &str) -> String {
    format!(
        "{}/variation/{}/{}?content-type=application/json",
        ensembl_base_url(),
        species,
        id
    )
}

fn ensembl_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (ensembl-integration-test)")
        .build()
}

#[derive(Debug, Deserialize)]
struct EnsemblGene {
    id: String,
    display_name: String,
    species: String,
    #[serde(default)]
    biotype: String,
    #[serde(default)]
    assembly_name: String,
    #[serde(default)]
    version: i64,
}

#[derive(Debug, Deserialize)]
struct EnsemblTranscript {
    id: String,
    display_name: Option<String>,
    species: String,
    #[serde(default)]
    biotype: String,
    #[serde(default, rename = "Parent")]
    gene_id: String,
    #[serde(default)]
    version: i64,
}

#[derive(Debug, Deserialize)]
struct EnsemblVariation {
    name: String,
    #[serde(default)]
    species: String,
    #[serde(default)]
    most_severe_consequence: String,
    #[serde(default)]
    minor_allele_freq: Option<f64>,
    #[serde(default, rename = "mappings")]
    mapping_results: Option<Vec<serde_json::Value>>,
}

async fn ensembl_fetch_gene(client: &Client, id: &str) -> Result<EnsemblGene, Box<dyn Error>> {
    let url = ensembl_lookup_id_url(id);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ensembl gene request failed for id '{id}' with status {status}: {body}"
        )
        .into());
    }
    let gene: EnsemblGene = response.json().await?;
    Ok(gene)
}

async fn ensembl_fetch_transcript(
    client: &Client,
    id: &str,
) -> Result<EnsemblTranscript, Box<dyn Error>> {
    let url = ensembl_lookup_id_url(id);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ensembl transcript request failed for id '{id}' with status {status}: {body}"
        )
        .into());
    }
    let tx: EnsemblTranscript = response.json().await?;
    Ok(tx)
}

async fn ensembl_fetch_variation(
    client: &Client,
    species: &str,
    id: &str,
) -> Result<EnsemblVariation, Box<dyn Error>> {
    let url = ensembl_variation_url(species, id);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ensembl variation request failed for {species}/{id} with status {status}: {body}"
        )
        .into());
    }
    let mut var: EnsemblVariation = response.json().await?;
    if var.species.trim().is_empty() {
        var.species = species.to_string();
    }
    Ok(var)
}

#[tokio::test]
async fn fetch_ensembl_brca2_gene() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let gene_id = "ENSG00000139618";
    let gene = ensembl_fetch_gene(&client, gene_id).await?;

    assert_eq!(
        gene.id, gene_id,
        "Expected Ensembl gene id '{}' for BRCA2, got '{}'",
        gene_id, gene.id
    );
    assert!(
        gene.display_name.eq_ignore_ascii_case("BRCA2"),
        "Expected Ensembl display_name 'BRCA2', got '{}'",
        gene.display_name
    );
    assert!(
        gene.species.eq_ignore_ascii_case("homo_sapiens"),
        "Expected Ensembl species 'homo_sapiens', got '{}'",
        gene.species
    );
    assert!(
        !gene.biotype.trim().is_empty(),
        "Expected a non-empty biotype for BRCA2"
    );
    assert!(
        !gene.assembly_name.trim().is_empty(),
        "Expected a non-empty assembly_name for BRCA2"
    );
    assert!(
        gene.version > 0,
        "Expected positive Ensembl gene version for BRCA2, got {}",
        gene.version
    );

    Ok(())
}

#[tokio::test]
async fn fetch_ensembl_brca2_transcript() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let tx_id = "ENST00000380152";
    let tx = ensembl_fetch_transcript(&client, tx_id).await?;

    assert_eq!(
        tx.id, tx_id,
        "Expected Ensembl transcript id '{}', got '{}'",
        tx_id, tx.id
    );
    assert!(
        tx.species.eq_ignore_ascii_case("homo_sapiens"),
        "Expected Ensembl species 'homo_sapiens', got '{}'",
        tx.species
    );
    assert!(
        !tx.gene_id.trim().is_empty(),
        "Expected transcript '{}' to have a non-empty gene_id",
        tx_id
    );
    assert!(
        !tx.biotype.trim().is_empty(),
        "Expected transcript '{}' to have a non-empty biotype",
        tx_id
    );
    if let Some(display_name) = tx.display_name.as_deref() {
        assert!(
            display_name.to_ascii_uppercase().contains("BRCA2"),
            "Expected transcript display_name to mention BRCA2, got '{}'",
            display_name
        );
    }
    assert!(
        tx.version > 0,
        "Expected positive transcript version for '{}', got {}",
        tx_id,
        tx.version
    );

    Ok(())
}

#[tokio::test]
async fn fetch_ensembl_brca2_variation() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let species = "homo_sapiens";
    let rs_id = "rs80359273";
    let var = ensembl_fetch_variation(&client, species, rs_id).await?;

    assert_eq!(
        var.name, rs_id,
        "Expected Ensembl variation name '{}' for BRCA2 variant, got '{}'",
        rs_id, var.name
    );
    assert!(
        var.species.eq_ignore_ascii_case(species),
        "Expected variation species '{}', got '{}'",
        species,
        var.species
    );
    assert!(
        !var.most_severe_consequence.trim().is_empty(),
        "Expected non-empty most_severe_consequence for {}",
        rs_id
    );
    if let Some(maf) = var.minor_allele_freq {
        assert!(
            (0.0..=1.0).contains(&maf),
            "Expected MAF in [0,1], got {}",
            maf
        );
    }
    if let Some(mapping_results) = &var.mapping_results {
        assert!(
            !mapping_results.is_empty(),
            "Expected non-empty mapping results when provided for {}",
            rs_id
        );
    }

    Ok(())
}

fn ensembl_sequence_url(id: &str) -> String {
    format!(
        "{}/sequence/id/{}?content-type=text/plain",
        ensembl_base_url(),
        id
    )
}

fn ensembl_assembly_info_url(species: &str) -> String {
    format!(
        "{}/info/assembly/{}?content-type=application/json",
        ensembl_base_url(),
        species
    )
}

#[derive(Debug, Deserialize)]
struct EnsemblAssemblyInfo {
    assembly_name: String,
    assembly_accession: String,
    #[serde(default)]
    species: String,
    #[serde(default)]
    karyotype: Vec<String>,
    #[serde(default)]
    default_coord_system_version: String,
}

#[derive(Debug, Deserialize)]
struct EnsemblProtein {
    id: String,
    species: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    version: i64,
    #[serde(default)]
    biotype: String,
}

async fn ensembl_fetch_assembly_info(
    client: &Client,
    species: &str,
) -> Result<EnsemblAssemblyInfo, Box<dyn Error>> {
    let url = ensembl_assembly_info_url(species);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ensembl assembly info request failed for species '{species}' with status {status}: {body}"
        )
        .into());
    }
    let mut info: EnsemblAssemblyInfo = response.json().await?;
    if info.species.trim().is_empty() {
        info.species = species.to_string();
    }
    Ok(info)
}

async fn ensembl_fetch_sequence(client: &Client, id: &str) -> Result<String, Box<dyn Error>> {
    let url = ensembl_sequence_url(id);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ensembl sequence request failed for id '{id}' with status {status}: {body}"
        )
        .into());
    }
    let seq = response.text().await?;
    Ok(seq)
}

async fn ensembl_fetch_protein(
    client: &Client,
    id: &str,
) -> Result<EnsemblProtein, Box<dyn Error>> {
    let url = ensembl_lookup_id_url(id);
    let response = client.get(&url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!(
            "Ensembl protein request failed for id '{id}' with status {status}: {body}"
        )
        .into());
    }
    let protein: EnsemblProtein = response.json().await?;
    Ok(protein)
}

fn is_dna_sequence(seq: &str) -> bool {
    seq.chars().all(|base| {
        matches!(
            base,
            'A' | 'C' | 'G' | 'T' | 'N' | 'a' | 'c' | 'g' | 't' | 'n'
        )
    })
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
async fn fetch_ensembl_human_assembly_info() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let species = "homo_sapiens";
    let info = ensembl_fetch_assembly_info(&client, species).await?;

    assert!(
        info.species.eq_ignore_ascii_case(species),
        "Expected assembly species '{}', got '{}'",
        species,
        info.species
    );
    assert!(
        info.assembly_name.to_ascii_uppercase().contains("GRCH38"),
        "Expected Ensembl assembly_name for human to contain 'GRCh38', got '{}'",
        info.assembly_name
    );
    assert!(
        !info.assembly_accession.trim().is_empty(),
        "Expected non-empty assembly_accession for human assembly"
    );
    assert!(
        !info.karyotype.is_empty(),
        "Expected non-empty karyotype list for human assembly"
    );
    assert!(
        !info.default_coord_system_version.trim().is_empty(),
        "Expected non-empty default_coord_system_version for human assembly"
    );

    Ok(())
}

#[tokio::test]
async fn fetch_ensembl_brca2_protein() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let protein_id = "ENSP00000419060";
    let protein = ensembl_fetch_protein(&client, protein_id).await?;

    assert_eq!(
        protein.id, protein_id,
        "Expected Ensembl protein id '{}', got '{}'",
        protein_id, protein.id
    );
    assert!(
        protein.species.eq_ignore_ascii_case("homo_sapiens"),
        "Expected Ensembl protein species 'homo_sapiens', got '{}'",
        protein.species
    );
    if let Some(name) = protein.display_name.as_deref() {
        assert!(
            name.to_ascii_uppercase().contains("BRCA2"),
            "Expected protein display_name to mention BRCA2, got '{}'",
            name
        );
    }
    assert!(
        protein.version > 0,
        "Expected positive Ensembl protein version for '{}', got {}",
        protein_id,
        protein.version
    );
    let _ = &protein.biotype;

    Ok(())
}

#[tokio::test]
async fn fetch_ensembl_brca2_gene_sequence() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let gene_id = "ENSG00000139618";
    let seq = ensembl_fetch_sequence(&client, gene_id).await?;

    assert!(
        !seq.trim().is_empty(),
        "Expected non-empty sequence for Ensembl gene '{}'",
        gene_id
    );
    assert!(
        is_dna_sequence(seq.trim()),
        "Expected DNA alphabet sequence for gene '{}', got leading snippet '{}'",
        gene_id,
        seq.chars().take(40).collect::<String>()
    );

    Ok(())
}

#[tokio::test]
async fn fetch_ensembl_brca2_transcript_sequence() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let tx_id = "ENST00000380152";
    let seq = ensembl_fetch_sequence(&client, tx_id).await?;

    assert!(
        !seq.trim().is_empty(),
        "Expected non-empty sequence for Ensembl transcript '{}'",
        tx_id
    );
    assert!(
        is_dna_sequence(seq.trim()),
        "Expected DNA alphabet sequence for transcript '{}', got leading snippet '{}'",
        tx_id,
        seq.chars().take(40).collect::<String>()
    );

    Ok(())
}

#[tokio::test]
async fn fetch_ensembl_brca2_protein_sequence() -> Result<(), Box<dyn Error>> {
    let client = ensembl_client()?;
    let protein_id = "ENSP00000419060";
    let seq = ensembl_fetch_sequence(&client, protein_id).await?;

    assert!(
        !seq.trim().is_empty(),
        "Expected non-empty sequence for Ensembl protein '{}'",
        protein_id
    );
    assert!(
        is_protein_sequence(seq.trim()),
        "Expected amino-acid alphabet sequence for protein '{}', got leading snippet '{}'",
        protein_id,
        seq.chars().take(40).collect::<String>()
    );

    Ok(())
}
