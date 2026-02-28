use anyhow::{Context, Result, anyhow, ensure};
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

const DEFAULT_USER_AGENT: &str = "cyto-vendor-examples/0.1 (integration-test)";

pub struct NcbiClient {
    client: Client,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strand {
    Forward,
    Reverse,
}

impl Strand {
    fn as_ncbi_param(self) -> u8 {
        match self {
            Strand::Forward => 1,
            Strand::Reverse => 2,
        }
    }
}

impl NcbiClient {
    pub fn new() -> Result<Self> {
        let client = Client::builder().user_agent(DEFAULT_USER_AGENT).build()?;
        Ok(Self { client })
    }

    /// Fetches a FASTA sequence for a region within a nuccore record.
    ///
    /// This is useful for retrieving genomic intervals from chromosome accessions
    /// like `NC_000013.11`.
    pub async fn fetch_nuccore_fasta_region(
        &self,
        id: &str,
        chr_start: u64,
        chr_stop: u64,
        strand: Strand,
    ) -> Result<String> {
        let url = ncbi_efetch_fasta_nuccore_region_url(id, chr_start, chr_stop, strand);
        self.fetch_text_with_retries(&url)
            .await
            .map_err(|err| anyhow!("Failed to fetch nuccore FASTA region from '{url}': {err}"))
    }

    pub async fn list_genes(
        &self,
        term: &str,
        offset: usize,
        limit: usize,
    ) -> Result<GeneSearchResult> {
        let url = ncbi_esearch_url("gene", term, limit, offset);
        let response: ESearchResponse = self.fetch_with_retries(&url).await?;
        let gene_ids = response
            .esearchresult
            .idlist
            .into_iter()
            .map(|id| {
                id.parse::<u64>().map_err(|err| {
                    anyhow!("Failed to parse NCBI gene id '{id}' from term '{term}': {err}")
                })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(GeneSearchResult {
            items: gene_ids,
            full_count: response.esearchresult.count,
            limit: response.esearchresult.retmax,
            offset: response.esearchresult.retstart,
        })
    }

    pub async fn fetch_gene(&self, gene_id: u64) -> Result<GeneMetadata> {
        let url = ncbi_esummary_url("gene", &gene_id.to_string());
        self.fetch_esummary_metadata("gene", &gene_id.to_string())
            .await
            .map_err(|err| anyhow!("Failed to fetch gene metadata from '{url}': {err}"))
    }

    pub async fn fetch_nuccore_metadata(&self, id: &str) -> Result<NuccoreMetadata> {
        self.fetch_esummary_metadata("nuccore", id)
            .await
            .map_err(|err| anyhow!("Failed to fetch nuccore metadata for id '{id}': {err}"))
    }

    pub async fn fetch_protein_metadata(&self, id: &str) -> Result<ProteinMetadata> {
        self.fetch_esummary_metadata("protein", id)
            .await
            .map_err(|err| anyhow!("Failed to fetch protein metadata for id '{id}': {err}"))
    }

    pub async fn fetch_assembly_metadata(&self, id: &str) -> Result<AssemblyMetadata> {
        self.fetch_esummary_metadata("assembly", id)
            .await
            .map_err(|err| anyhow!("Failed to fetch assembly metadata for id '{id}': {err}"))
    }

    pub async fn fetch_sra_metadata(&self, id: &str) -> Result<SraMetadata> {
        let raw: RawSraMetadata = self
            .fetch_esummary_metadata("sra", id)
            .await
            .map_err(|err| anyhow!("Failed to fetch SRA metadata for id '{id}': {err}"))?;

        let title = extract_xml_tag_text(&raw.expxml, "Title").ok_or_else(|| {
            anyhow!(
                "SRA expxml did not contain a <Title> element for id '{id}'. expxml='{}'",
                raw.expxml
            )
        })?;
        let study_accession =
            extract_xml_attribute(&raw.expxml, "Study", "acc").ok_or_else(|| {
                anyhow!(
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

    pub async fn fetch_pubmed_metadata(&self, id: &str) -> Result<PubmedMetadata> {
        self.fetch_esummary_metadata("pubmed", id)
            .await
            .map_err(|err| anyhow!("Failed to fetch PubMed metadata for id '{id}': {err}"))
    }

    pub async fn fetch_taxonomy_metadata(&self, taxid: &str) -> Result<TaxonomyMetadata> {
        self.fetch_esummary_metadata("taxonomy", taxid)
            .await
            .map_err(|err| anyhow!("Failed to fetch taxonomy metadata for taxid '{taxid}': {err}"))
    }

    pub async fn fetch_clinvar_metadata(&self, id: &str) -> Result<ClinvarMetadata> {
        let mut metadata: ClinvarMetadata = self
            .fetch_esummary_metadata("clinvar", id)
            .await
            .map_err(|err| anyhow!("Failed to fetch ClinVar metadata for id '{id}': {err}"))?;

        if metadata.clinicalsignificancetext.trim().is_empty() {
            metadata.clinicalsignificancetext =
                metadata.germline_classification.description.clone();
        }
        if metadata.variationid.trim().is_empty() {
            metadata.variationid = metadata.accession.clone();
        }

        Ok(metadata)
    }

    pub async fn fetch_biosample_metadata(&self, accession: &str) -> Result<BiosampleMetadata> {
        self.fetch_esummary_metadata("biosample", accession)
            .await
            .map_err(|err| {
                anyhow!("Failed to fetch BioSample metadata for accession '{accession}': {err}")
            })
    }

    pub async fn fetch_bioproject_metadata(&self, accession: &str) -> Result<BioprojectMetadata> {
        self.fetch_esummary_metadata("bioproject", accession)
            .await
            .map_err(|err| {
                anyhow!("Failed to fetch BioProject metadata for accession '{accession}': {err}")
            })
    }

    pub async fn fetch_mesh_metadata(&self, id: &str) -> Result<MeshMetadata> {
        self.fetch_esummary_metadata("mesh", id)
            .await
            .map_err(|err| anyhow!("Failed to fetch MeSH metadata for id '{id}': {err}"))
    }

    async fn fetch_esummary_metadata<T>(&self, db: &str, id: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let url = ncbi_esummary_url(db, id);
        let mut payload: ESummaryResponse<T> = self.fetch_with_retries(&url).await?;
        let uid = payload
            .result
            .uids
            .first()
            .cloned()
            .ok_or_else(|| anyhow!("NCBI response for URL '{url}' did not contain any UIDs"))?;

        let metadata = payload.result.records.remove(&uid).ok_or_else(|| {
            anyhow!("NCBI response for URL '{url}' did not contain record for uid '{uid}'")
        })?;

        Ok(metadata)
    }

    async fn fetch_with_retries<T>(&self, url: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut last_rate_limit_error = None;

        for attempt in 0..5 {
            let response = {
                let _guard = ncbi_request_gate().lock().await;
                sleep(Duration::from_millis(350)).await;
                self.client.get(url).send().await?
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
                return Err(anyhow!(
                    "NCBI request failed for URL '{url}' with status {status}. Response body: {body}"
                ));
            }

            let payload = response.json::<serde_json::Value>().await?;
            println!("NCBI response payload for URL '{url}': {payload:#?}");
            let payload = serde_json::from_value::<T>(payload)?;
            return Ok(payload);
        }

        Err(anyhow!(
            "{}",
            last_rate_limit_error
                .unwrap_or_else(|| format!("NCBI request retries exhausted for URL '{url}'"))
        ))
    }

    async fn fetch_text_with_retries(&self, url: &str) -> Result<String> {
        let mut last_rate_limit_error = None;

        for attempt in 0..5 {
            let response = {
                let _guard = ncbi_request_gate().lock().await;
                sleep(Duration::from_millis(350)).await;
                self.client.get(url).send().await?
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
                return Err(anyhow!(
                    "NCBI request failed for URL '{url}' with status {status}. Response body: {body}"
                ));
            }

            return Ok(response.text().await?);
        }

        Err(anyhow!(
            "{}",
            last_rate_limit_error
                .unwrap_or_else(|| format!("NCBI request retries exhausted for URL '{url}'"))
        ))
    }
}

fn ncbi_request_gate() -> &'static Mutex<()> {
    static REQUEST_GATE: OnceLock<Mutex<()>> = OnceLock::new();
    REQUEST_GATE.get_or_init(|| Mutex::new(()))
}

fn ncbi_esummary_url(db: &str, id: &str) -> String {
    format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db={db}&id={id}&retmode=json"
    )
}

fn ncbi_esearch_url(db: &str, term: &str, retmax: usize, retstart: usize) -> String {
    format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db={db}&term={term}&retmode=json&retmax={retmax}&retstart={retstart}"
    )
}

fn ncbi_efetch_fasta_nuccore_region_url(
    id: &str,
    chr_start: u64,
    chr_stop: u64,
    strand: Strand,
) -> String {
    format!(
        "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/efetch.fcgi?db=nuccore&id={id}&rettype=fasta&retmode=text&strand={}&seq_start={chr_start}&seq_stop={chr_stop}",
        strand.as_ncbi_param()
    )
}

fn deserialize_u64_from_string_or_number<'de, D>(
    deserializer: D,
) -> std::result::Result<u64, D::Error>
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

pub fn parse_fasta(fasta: &str) -> Result<(String, String)> {
    let mut lines = fasta.lines();
    let header_line = lines.next().context("FASTA was empty")?.trim_end();
    ensure!(
        header_line.starts_with('>'),
        "FASTA header did not start with '>', got '{header_line}'"
    );

    let header = header_line.trim_start_matches('>').trim().to_string();
    ensure!(!header.is_empty(), "FASTA header was empty");

    let mut sequence = String::new();
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        ensure!(
            !line.starts_with('>'),
            "FASTA contained an unexpected second header: '{line}'"
        );
        sequence.push_str(line);
    }

    ensure!(!sequence.is_empty(), "FASTA sequence was empty");

    // Accept IUPAC nucleotide codes (including N) in either case.
    for ch in sequence.chars() {
        let ch = ch.to_ascii_uppercase();
        let ok = matches!(
            ch,
            'A' | 'C'
                | 'G'
                | 'T'
                | 'U'
                | 'R'
                | 'Y'
                | 'K'
                | 'M'
                | 'S'
                | 'W'
                | 'B'
                | 'D'
                | 'H'
                | 'V'
                | 'N'
        );
        ensure!(ok, "FASTA sequence contained an invalid base '{ch}'");
    }

    Ok((header, sequence))
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
struct ESearchResponse {
    esearchresult: ESearchResult,
}

#[derive(Debug, Deserialize)]
struct ESearchResult {
    idlist: Vec<String>,

    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    count: u64,

    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    retmax: u64,

    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    retstart: u64,
}

#[derive(Debug, Deserialize)]
pub struct GeneSearchResult {
    pub items: Vec<u64>,
    pub full_count: u64,
    pub limit: u64,
    pub offset: u64,
}

#[derive(Debug, Deserialize)]
pub struct GeneMetadata {
    #[serde(
        rename = "uid",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub gene_id: u64,
    pub description: String,
    pub summary: String,
    #[serde(rename = "nomenclaturesymbol")]
    pub symbol: String,
    pub chromosome: String,
    pub organism: Organism,
    #[serde(rename = "locationhist")]
    pub location_hist: Vec<LocationHist>,
}

#[derive(Debug, Deserialize)]
pub struct LocationHist {
    #[serde(rename = "annotationrelease")]
    pub annotation_release: String,
    #[serde(rename = "assemblyaccver")]
    pub assembly_acc_ver: String,
    #[serde(rename = "chraccver")]
    pub chr_acc_ver: String,
    #[serde(rename = "chrstart")]
    pub chr_start: u64,
    #[serde(rename = "chrstop")]
    pub chr_stop: u64,
}

#[derive(Debug, Deserialize)]
pub struct NuccoreMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    pub caption: String,
    pub title: String,
    #[serde(
        rename = "slen",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub length: u64,
}

#[derive(Debug, Deserialize)]
pub struct ProteinMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    pub caption: String,
    pub title: String,
    #[serde(
        rename = "slen",
        deserialize_with = "deserialize_u64_from_string_or_number"
    )]
    pub length: u64,
}

#[derive(Debug, Deserialize)]
pub struct AssemblyMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    pub assemblyaccession: String,
    pub assemblyname: String,
    pub organism: String,
    #[serde(default)]
    pub assemblystatus: String,
    #[serde(default)]
    pub releaselevel: String,
}

#[derive(Debug, Deserialize)]
pub struct SraMetadata {
    pub uid: u64,
    pub title: String,
    pub study_accession: String,
    pub expxml: String,
    pub runs: String,
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
pub struct PubmedMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    pub title: String,
    pub source: String,
    #[serde(default)]
    pub pubdate: String,
}

#[derive(Debug, Deserialize)]
pub struct TaxonomyMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    #[serde(rename = "scientificname")]
    pub scientific_name: String,
    #[serde(default)]
    pub rank: String,
}

#[derive(Debug, Deserialize)]
pub struct ClinvarMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub clinicalsignificancetext: String,
    #[serde(default)]
    pub variationid: String,
    #[serde(default)]
    pub accession: String,
    #[serde(default)]
    germline_classification: ClinvarGermlineClassification,
}

#[derive(Debug, Default, Deserialize)]
struct ClinvarGermlineClassification {
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
pub struct BiosampleMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    pub accession: String,
    pub title: String,
    #[serde(default)]
    pub organism: String,
}

#[derive(Debug, Deserialize)]
pub struct BioprojectMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    #[serde(rename = "project_acc")]
    pub project_accession: String,
    #[serde(rename = "project_title")]
    pub title: String,
    #[serde(default, rename = "organism_name")]
    pub organism: String,
}

#[derive(Debug, Deserialize)]
pub struct MeshMetadata {
    #[serde(deserialize_with = "deserialize_u64_from_string_or_number")]
    pub uid: u64,
    #[serde(rename = "ds_meshui")]
    pub mesh_id: String,
    #[serde(rename = "ds_meshterms", deserialize_with = "deserialize_mesh_heading")]
    pub heading: String,
    #[serde(
        default,
        rename = "ds_idxlinks",
        deserialize_with = "deserialize_mesh_tree_numbers"
    )]
    pub tree_numbers: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct MeshIndexLink {
    #[serde(default)]
    treenum: String,
}

#[derive(Debug, Deserialize)]
pub struct Organism {
    #[serde(rename = "taxid")]
    pub taxon_id: u64,
    #[serde(rename = "scientificname")]
    pub scientific_name: String,
    #[serde(rename = "commonname")]
    pub common_name: String,
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

fn deserialize_mesh_heading<'de, D>(deserializer: D) -> std::result::Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let terms = Vec::<String>::deserialize(deserializer)?;
    terms
        .into_iter()
        .find(|term| !term.trim().is_empty())
        .ok_or_else(|| serde::de::Error::custom("ds_meshterms did not contain a heading"))
}

fn deserialize_mesh_tree_numbers<'de, D>(
    deserializer: D,
) -> std::result::Result<Vec<String>, D::Error>
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
