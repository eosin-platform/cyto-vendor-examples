use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::error::Error;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

fn ncbi_esummary_url(db: &str, id: &str) -> String {
    format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db={db}&id={id}&retmode=json"
    )
}

fn ncbi_gene_url(gene_id: u64) -> String {
    ncbi_esummary_url("gene", &gene_id.to_string())
}

fn ncbi_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (integration-test)")
        .build()
}

fn ncbi_request_gate() -> &'static Mutex<()> {
    static REQUEST_GATE: OnceLock<Mutex<()>> = OnceLock::new();
    REQUEST_GATE.get_or_init(|| Mutex::new(()))
}

fn deserialize_u64_from_string_or_number<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum U64Like {
        String(String),
        Number(u64),
    }

    match U64Like::deserialize(deserializer)? {
        U64Like::String(value) => value.parse::<u64>().map_err(serde::de::Error::custom),
        U64Like::Number(value) => Ok(value),
    }
}

#[derive(Debug, Deserialize)]
struct ESummaryResponse<T> {
    result: ESummaryResult<T>,
}

#[derive(Debug, Deserialize)]
struct ESummaryResult<T> {
    uids: Vec<String>,
    #[serde(flatten)]
    records: HashMap<String, T>,
}

#[derive(Debug, Deserialize)]
struct GeneMetadata {
    #[serde(
        rename = "uid",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    gene_id: u64,
    #[serde(rename = "nomenclaturesymbol")]
    symbol: String,
    organism: Organism,
}

#[derive(Debug, Deserialize)]
struct NuccoreMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    caption: String,
    title: String,
    #[serde(
        rename = "slen",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    length: u64,
}

#[derive(Debug, Deserialize)]
struct ProteinMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    caption: String,
    title: String,
    #[serde(
        rename = "slen",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    length: u64,
}

#[derive(Debug, Deserialize)]
struct AssemblyMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    assemblyaccession: String,
    assemblyname: String,
    organism: String,
    #[serde(default)]
    assemblystatus: String,
    #[serde(default)]
    releaselevel: String,
}

#[derive(Debug, Deserialize)]
struct SraMetadata {
    uid: u64,
    title: String,
    study_accession: String,
    expxml: String,
    runs: String,
}

#[derive(Debug, Deserialize)]
struct RawSraMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    #[serde(default)]
    expxml: String,
    #[serde(default)]
    runs: String,
}

#[derive(Debug, Deserialize)]
struct PubmedMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    title: String,
    source: String,
    #[serde(default)]
    pubdate: String,
}

#[derive(Debug, Deserialize)]
struct TaxonomyMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    #[serde(rename = "scientificname")]
    scientific_name: String,
    #[serde(default)]
    rank: String,
}

#[derive(Debug, Deserialize)]
struct ClinvarMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    #[serde(default)]
    title: String,
    #[serde(default)]
    clinicalsignificancetext: String,
    #[serde(default)]
    variationid: String,
    #[serde(default)]
    accession: String,
    #[serde(default)]
    germline_classification: ClinvarGermlineClassification,
}

#[derive(Debug, Default, Deserialize)]
struct ClinvarGermlineClassification {
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct BiosampleMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    accession: String,
    title: String,
    #[serde(default)]
    organism: String,
}

#[derive(Debug, Deserialize)]
struct BioprojectMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    #[serde(rename = "project_acc")]
    project_accession: String,
    #[serde(rename = "project_title")]
    title: String,
    #[serde(default, rename = "organism_name")]
    organism: String,
}

#[derive(Debug, Deserialize)]
struct MeshMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    uid: u64,
    #[serde(rename = "ds_meshui")]
    mesh_id: String,
    #[serde(rename = "ds_meshterms", deserialize_with = "deserialize_mesh_heading")]
    heading: String,
    #[serde(
        default,
        rename = "ds_idxlinks",
        deserialize_with = "deserialize_mesh_tree_numbers"
    )]
    tree_numbers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MeshIndexLink {
    #[serde(default)]
    treenum: String,
}

#[derive(Debug, Deserialize)]
struct Organism {
    #[serde(rename = "taxid")]
    taxon_id: u64,
    #[serde(rename = "scientificname")]
    scientific_name: String,
}

fn extract_xml_tag_text(xml: &str, tag: &str) -> Option<String> {
    let start_token = format!("<{tag}>");
    let end_token = format!("</{tag}>");
    let start = xml.find(&start_token)? + start_token.len();
    let end = xml[start..].find(&end_token)? + start;
    Some(xml[start..end].trim().to_string())
}

fn extract_xml_attribute(xml: &str, element: &str, attribute: &str) -> Option<String> {
    let element_token = format!("<{element} ");
    let start = xml.find(&element_token)?;
    let after_element = &xml[start..];
    let close = after_element.find('>')?;
    let element_header = &after_element[..close];
    let attr_token = format!(r#"{attribute}=""#);
    let attr_start = element_header.find(&attr_token)? + attr_token.len();
    let remainder = &element_header[attr_start..];
    let attr_end = remainder.find('"')?;
    Some(remainder[..attr_end].to_string())
}

fn deserialize_mesh_heading<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let terms = Vec::<String>::deserialize(deserializer)?;
    terms
        .into_iter()
        .find(|term| !term.trim().is_empty())
        .ok_or_else(|| serde::de::Error::custom("ds_meshterms did not contain a heading"))
}

fn deserialize_mesh_tree_numbers<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let links = Vec::<MeshIndexLink>::deserialize(deserializer)?;
    Ok(links
        .into_iter()
        .filter_map(|link| {
            let tree_number = link.treenum.trim();
            if tree_number.is_empty() {
                None
            } else {
                Some(tree_number.to_string())
            }
        })
        .collect())
}

async fn fetch_esummary_metadata<T>(
    client: &Client,
    db: &str,
    id: &str,
) -> Result<T, Box<dyn Error>>
where
    T: DeserializeOwned,
{
    let url = ncbi_esummary_url(db, id);
    let mut last_rate_limit_error = None;

    for attempt in 0..5 {
        let response = {
            let _guard = ncbi_request_gate().lock().await;
            sleep(Duration::from_millis(350)).await;
            client.get(&url).send().await?
        };

        let status = response.status();
        if status.as_u16() == 429 {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            last_rate_limit_error = Some(format!(
                "NCBI rate limit (429) for URL '{url}' on attempt {}. Response body: {body}",
                attempt + 1
            ));
            sleep(Duration::from_secs(1 + attempt)).await;
            continue;
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_else(|_| String::new());
            return Err(format!(
                "NCBI request failed for URL '{url}' with status {status}. Response body: {body}"
            )
            .into());
        }

        let mut payload: ESummaryResponse<T> = response.json().await?;
        let uid = payload
            .result
            .uids
            .first()
            .cloned()
            .ok_or_else(|| format!("NCBI response for URL '{url}' did not contain any UIDs"))?;

        let metadata = payload.result.records.remove(&uid).ok_or_else(|| {
            format!("NCBI response for URL '{url}' did not contain record for uid '{uid}'")
        })?;

        return Ok(metadata);
    }

    Err(last_rate_limit_error
        .unwrap_or_else(|| format!("NCBI request retries exhausted for URL '{url}'"))
        .into())
}

async fn fetch_gene_metadata(
    client: &Client,
    gene_id: u64,
) -> Result<GeneMetadata, Box<dyn Error>> {
    let url = ncbi_gene_url(gene_id);
    fetch_esummary_metadata(client, "gene", &gene_id.to_string())
        .await
        .map_err(|err| format!("Failed to fetch gene metadata from '{url}': {err}").into())
}

async fn nuccore_fetch_metadata(
    client: &Client,
    id: &str,
) -> Result<NuccoreMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "nuccore", id)
        .await
        .map_err(|err| format!("Failed to fetch nuccore metadata for id '{id}': {err}").into())
}

async fn protein_fetch_metadata(
    client: &Client,
    id: &str,
) -> Result<ProteinMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "protein", id)
        .await
        .map_err(|err| format!("Failed to fetch protein metadata for id '{id}': {err}").into())
}

async fn assembly_fetch_metadata(
    client: &Client,
    id: &str,
) -> Result<AssemblyMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "assembly", id)
        .await
        .map_err(|err| format!("Failed to fetch assembly metadata for id '{id}': {err}").into())
}

async fn sra_fetch_metadata(client: &Client, id: &str) -> Result<SraMetadata, Box<dyn Error>> {
    let raw: RawSraMetadata = fetch_esummary_metadata(client, "sra", id)
        .await
        .map_err(|err| format!("Failed to fetch SRA metadata for id '{id}': {err}"))?;

    let title = extract_xml_tag_text(&raw.expxml, "Title").ok_or_else(|| {
        format!(
            "SRA expxml did not contain a <Title> element for id '{id}'. expxml='{}'",
            raw.expxml
        )
    })?;
    let study_accession = extract_xml_attribute(&raw.expxml, "Study", "acc").ok_or_else(|| {
        format!(
            "SRA expxml did not contain Study acc attribute for id '{id}'. expxml='{}'",
            raw.expxml
        )
    })?;

    Ok(SraMetadata {
        uid: raw.uid,
        title,
        study_accession,
        expxml: raw.expxml,
        runs: raw.runs,
    })
}

async fn pubmed_fetch_metadata(
    client: &Client,
    id: &str,
) -> Result<PubmedMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "pubmed", id)
        .await
        .map_err(|err| format!("Failed to fetch PubMed metadata for id '{id}': {err}").into())
}

async fn taxonomy_fetch_metadata(
    client: &Client,
    taxid: &str,
) -> Result<TaxonomyMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "taxonomy", taxid)
        .await
        .map_err(|err| {
            format!("Failed to fetch taxonomy metadata for taxid '{taxid}': {err}").into()
        })
}

async fn clinvar_fetch_metadata(
    client: &Client,
    id: &str,
) -> Result<ClinvarMetadata, Box<dyn Error>> {
    let mut metadata: ClinvarMetadata = fetch_esummary_metadata(client, "clinvar", id)
        .await
        .map_err(|err| format!("Failed to fetch ClinVar metadata for id '{id}': {err}"))?;

    if metadata.clinicalsignificancetext.trim().is_empty() {
        metadata.clinicalsignificancetext = metadata.germline_classification.description.clone();
    }
    if metadata.variationid.trim().is_empty() {
        metadata.variationid = metadata.accession.clone();
    }

    Ok(metadata)
}

async fn biosample_fetch_metadata(
    client: &Client,
    accession: &str,
) -> Result<BiosampleMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "biosample", accession)
        .await
        .map_err(|err| {
            format!("Failed to fetch BioSample metadata for accession '{accession}': {err}").into()
        })
}

async fn bioproject_fetch_metadata(
    client: &Client,
    accession: &str,
) -> Result<BioprojectMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "bioproject", accession)
        .await
        .map_err(|err| {
            format!("Failed to fetch BioProject metadata for accession '{accession}': {err}").into()
        })
}

async fn mesh_fetch_metadata(client: &Client, id: &str) -> Result<MeshMetadata, Box<dyn Error>> {
    fetch_esummary_metadata(client, "mesh", id)
        .await
        .map_err(|err| format!("Failed to fetch MeSH metadata for id '{id}': {err}").into())
}

#[tokio::test]
async fn fetch_brca2_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = fetch_gene_metadata(&client, 675).await?;

    assert_eq!(
        metadata.gene_id, 675,
        "Expected NCBI gene ID 675 for BRCA2, got {}",
        metadata.gene_id
    );
    assert_eq!(
        metadata.symbol, "BRCA2",
        "Expected symbol 'BRCA2' for gene 675, got '{}'",
        metadata.symbol
    );
    assert_eq!(
        metadata.organism.taxon_id, 9606,
        "Expected taxon ID 9606 (Homo sapiens) for BRCA2, got {}",
        metadata.organism.taxon_id
    );
    assert!(
        metadata
            .organism
            .scientific_name
            .eq_ignore_ascii_case("Homo sapiens"),
        "Expected scientific name 'Homo sapiens' for BRCA2, got '{}'",
        metadata.organism.scientific_name
    );

    Ok(())
}

#[tokio::test]
async fn fetch_brca2_nuccore_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = nuccore_fetch_metadata(&client, "NM_000059.4").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive nuccore UID for NM_000059.4, got {}",
        metadata.uid
    );
    assert!(
        metadata.caption.contains("NM_000059"),
        "Expected nuccore caption to contain 'NM_000059', got '{}'",
        metadata.caption
    );
    assert!(
        metadata.length > 0,
        "Expected nuccore sequence length to be > 0, got {}",
        metadata.length
    );
    assert!(
        metadata.title.to_ascii_lowercase().contains("brca2"),
        "Expected nuccore title to mention BRCA2, got '{}'",
        metadata.title
    );

    Ok(())
}

#[tokio::test]
async fn fetch_brca2_protein_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = protein_fetch_metadata(&client, "NP_000050.3").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive protein UID for NP_000050.3, got {}",
        metadata.uid
    );
    assert!(
        metadata.caption.contains("NP_000050"),
        "Expected protein accession caption to contain 'NP_000050', got '{}'",
        metadata.caption
    );
    assert!(
        metadata.length > 0,
        "Expected protein length to be > 0, got {}",
        metadata.length
    );
    let protein_title = metadata.title.to_ascii_lowercase();
    assert!(
        protein_title.contains("brca2") || protein_title.contains("breast cancer type 2"),
        "Expected protein title to mention BRCA2 or 'breast cancer type 2', got '{}'",
        metadata.title
    );

    Ok(())
}

#[tokio::test]
async fn fetch_human_assembly_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = assembly_fetch_metadata(&client, "11968211").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive assembly UID for 11968211 (GRCh38), got {}",
        metadata.uid
    );
    assert!(
        metadata.assemblyaccession.starts_with("GCF_000001405"),
        "Expected assembly accession to start with 'GCF_000001405', got '{}'",
        metadata.assemblyaccession
    );
    assert!(
        metadata
            .organism
            .to_ascii_lowercase()
            .contains("homo sapiens"),
        "Expected assembly organism Homo sapiens, got '{}'",
        metadata.organism
    );
    assert!(
        metadata.assemblyname.contains("GRCh38"),
        "Expected assembly name to contain 'GRCh38', got '{}'",
        metadata.assemblyname
    );
    let level_text = format!(
        "{} {}",
        metadata.assemblystatus.to_ascii_lowercase(),
        metadata.releaselevel.to_ascii_lowercase()
    );
    assert!(
        level_text.contains("chromosome"),
        "Expected assembly level/status to indicate chromosome-level assembly, got assemblystatus='{}', releaselevel='{}'",
        metadata.assemblystatus,
        metadata.releaselevel
    );

    Ok(())
}

#[tokio::test]
async fn fetch_example_sra_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = sra_fetch_metadata(&client, "7426").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive SRA UID for 7426 (SRP001027-linked record), got {}",
        metadata.uid
    );
    assert!(
        !metadata.title.trim().is_empty(),
        "Expected non-empty SRA title for UID 7426"
    );
    let title_lower = metadata.title.to_ascii_lowercase();
    let xml_lower = metadata.expxml.to_ascii_lowercase();
    assert!(
        title_lower.contains("genome") || xml_lower.contains("genome"),
        "Expected SRA title/XML to contain known substring 'genome'. title='{}'",
        metadata.title
    );
    assert!(
        metadata.study_accession.starts_with("SRP"),
        "Expected SRA study accession to start with 'SRP', got '{}'",
        metadata.study_accession
    );
    assert!(
        !metadata.runs.trim().is_empty() || !metadata.expxml.trim().is_empty(),
        "Expected SRA metadata to include non-empty runs or expxml payload"
    );

    Ok(())
}

#[tokio::test]
async fn fetch_example_pubmed_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = pubmed_fetch_metadata(&client, "31452104").await?;

    assert_eq!(
        metadata.uid, 31_452_104,
        "Expected PubMed UID (PMID) 31452104, got {}",
        metadata.uid
    );
    assert!(
        !metadata.title.trim().is_empty(),
        "Expected non-empty PubMed title for PMID 31452104"
    );
    assert!(
        !metadata.source.trim().is_empty(),
        "Expected non-empty PubMed source/journal for PMID 31452104"
    );
    assert!(
        !metadata.pubdate.trim().is_empty(),
        "Expected non-empty PubMed publication date for PMID 31452104"
    );

    Ok(())
}

#[tokio::test]
async fn fetch_human_taxonomy_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = taxonomy_fetch_metadata(&client, "9606").await?;

    assert_eq!(
        metadata.uid, 9606,
        "Expected taxonomy uid 9606 for Homo sapiens, got {}",
        metadata.uid
    );
    assert!(
        metadata
            .scientific_name
            .eq_ignore_ascii_case("Homo sapiens"),
        "Expected scientific name 'Homo sapiens', got '{}'",
        metadata.scientific_name
    );
    assert!(
        metadata.rank.eq_ignore_ascii_case("species"),
        "Expected taxonomy rank 'species' for 9606, got '{}'",
        metadata.rank
    );

    Ok(())
}

#[tokio::test]
async fn fetch_example_clinvar_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = clinvar_fetch_metadata(&client, "17875").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive ClinVar UID for id 17875, got {}",
        metadata.uid
    );
    assert!(
        !metadata.title.trim().is_empty(),
        "Expected non-empty ClinVar title for id 17875"
    );
    assert!(
        !metadata.variationid.trim().is_empty(),
        "Expected non-empty ClinVar variationid/accession for id 17875"
    );
    let significance = metadata.clinicalsignificancetext.to_ascii_lowercase();
    assert!(
        significance.contains("pathogenic") || significance.contains("likely pathogenic"),
        "Expected clinical significance to mention pathogenicity, got '{}'",
        metadata.clinicalsignificancetext
    );

    Ok(())
}

#[tokio::test]
async fn fetch_example_biosample_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = biosample_fetch_metadata(&client, "2876").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive BioSample UID for SAMN00002876 (uid 2876), got {}",
        metadata.uid
    );
    assert_eq!(
        metadata.accession, "SAMN00002876",
        "Expected BioSample accession SAMN00002876, got '{}'",
        metadata.accession
    );
    assert!(
        !metadata.title.trim().is_empty(),
        "Expected non-empty BioSample title for SAMN00002876"
    );
    assert!(
        metadata
            .organism
            .to_ascii_lowercase()
            .contains("xanthomonas vasicola"),
        "Expected BioSample organism to contain 'Xanthomonas vasicola', got '{}'",
        metadata.organism
    );

    Ok(())
}

#[tokio::test]
async fn fetch_example_bioproject_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = bioproject_fetch_metadata(&client, "31213").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive BioProject UID for PRJNA31213 (uid 31213), got {}",
        metadata.uid
    );
    assert_eq!(
        metadata.project_accession, "PRJNA31213",
        "Expected BioProject accession PRJNA31213, got '{}'",
        metadata.project_accession
    );
    assert!(
        !metadata.title.trim().is_empty(),
        "Expected non-empty BioProject title for PRJNA31213"
    );
    assert!(
        metadata
            .organism
            .to_ascii_lowercase()
            .contains("xanthomonas vasicola"),
        "Expected BioProject organism to contain 'Xanthomonas vasicola', got '{}'",
        metadata.organism
    );

    Ok(())
}

#[tokio::test]
async fn fetch_mesh_breast_neoplasms_from_ncbi() -> Result<(), Box<dyn Error>> {
    let client = ncbi_client()?;
    let metadata = mesh_fetch_metadata(&client, "68001943").await?;

    assert!(
        metadata.uid > 0,
        "Expected positive MeSH UID for Breast Neoplasms, got {}",
        metadata.uid
    );
    assert_eq!(
        metadata.mesh_id, "D001943",
        "Expected MeSH ID 'D001943' for Breast Neoplasms, got '{}'",
        metadata.mesh_id
    );
    assert!(
        metadata.heading.eq_ignore_ascii_case("Breast Neoplasms"),
        "Expected MeSH heading 'Breast Neoplasms', got '{}'",
        metadata.heading
    );
    assert!(
        !metadata.tree_numbers.is_empty(),
        "Expected at least one MeSH tree number for D001943"
    );
    assert!(
        metadata
            .tree_numbers
            .iter()
            .any(|tree_number| tree_number.starts_with("C04") || tree_number.starts_with('C')),
        "Expected at least one tree number under C04 (Neoplasms) or C, got {:?}",
        metadata.tree_numbers
    );

    Ok(())
}
