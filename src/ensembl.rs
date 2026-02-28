use anyhow::{Result, anyhow};
use reqwest::Client;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use std::sync::OnceLock;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

const DEFAULT_USER_AGENT: &str = "cyto-vendor-examples/0.1 (integration-test)";
const ENSEMBL_REST_BASE_URL: &str = "https://rest.ensembl.org";

pub struct EnsemblClient {
    client: Client,
    base_url: String,
}

impl EnsemblClient {
    pub fn new() -> Result<Self> {
        Self::with_base_url(ENSEMBL_REST_BASE_URL)
    }

    pub fn with_base_url(base_url: &str) -> Result<Self> {
        let client = Client::builder().user_agent(DEFAULT_USER_AGENT).build()?;
        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn fetch_gene(&self, id: &str) -> Result<EnsemblGene> {
        let url = self.lookup_id_url(id);
        self.fetch_json_with_retries(&url).await.map_err(|err| {
            anyhow!("Failed to fetch Ensembl gene for id '{id}' from '{url}': {err}")
        })
    }

    pub async fn fetch_transcript(&self, id: &str) -> Result<EnsemblTranscript> {
        let url = self.lookup_id_url(id);
        self.fetch_json_with_retries(&url).await.map_err(|err| {
            anyhow!("Failed to fetch Ensembl transcript for id '{id}' from '{url}': {err}")
        })
    }

    pub async fn fetch_protein(&self, id: &str) -> Result<EnsemblProtein> {
        let url = self.lookup_id_url(id);
        self.fetch_json_with_retries(&url).await.map_err(|err| {
            anyhow!("Failed to fetch Ensembl protein for id '{id}' from '{url}': {err}")
        })
    }

    pub async fn fetch_variation(&self, species: &str, id: &str) -> Result<EnsemblVariation> {
        let url = self.variation_url(species, id);
        let mut var: EnsemblVariation =
            self.fetch_json_with_retries(&url).await.map_err(|err| {
                anyhow!("Failed to fetch Ensembl variation for {species}/{id} from '{url}': {err}")
            })?;

        if var.species.trim().is_empty() {
            var.species = species.to_string();
        }

        Ok(var)
    }

    pub async fn fetch_assembly_info(&self, species: &str) -> Result<EnsemblAssemblyInfo> {
        let url = self.assembly_info_url(species);
        let mut info: EnsemblAssemblyInfo = self
            .fetch_json_with_retries(&url)
            .await
            .map_err(|err| anyhow!(
                "Failed to fetch Ensembl assembly info for species '{species}' from '{url}': {err}"
            ))?;

        if info.species.trim().is_empty() {
            info.species = species.to_string();
        }

        Ok(info)
    }

    pub async fn fetch_sequence(&self, id: &str) -> Result<String> {
        let url = self.sequence_url(id);
        self.fetch_text_with_retries(&url).await.map_err(|err| {
            anyhow!("Failed to fetch Ensembl sequence for id '{id}' from '{url}': {err}")
        })
    }

    pub async fn fetch_fasta(&self, id: &str) -> Result<String> {
        let url = self.fasta_url(id);
        self.fetch_text_with_retries(&url).await.map_err(|err| {
            anyhow!("Failed to fetch Ensembl FASTA for id '{id}' from '{url}': {err}")
        })
    }

    fn lookup_id_url(&self, id: &str) -> String {
        format!(
            "{}/lookup/id/{}?content-type=application/json",
            self.base_url, id
        )
    }

    fn variation_url(&self, species: &str, id: &str) -> String {
        format!(
            "{}/variation/{}/{}?content-type=application/json",
            self.base_url, species, id
        )
    }

    fn assembly_info_url(&self, species: &str) -> String {
        format!(
            "{}/info/assembly/{}?content-type=application/json",
            self.base_url, species
        )
    }

    fn sequence_url(&self, id: &str) -> String {
        format!(
            "{}/sequence/id/{}?content-type=text/plain",
            self.base_url, id
        )
    }

    fn fasta_url(&self, id: &str) -> String {
        format!(
            "{}/sequence/id/{}?content-type=text/x-fasta",
            self.base_url, id
        )
    }

    async fn fetch_json_with_retries<T>(&self, url: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let mut last_retryable_error = None;

        for attempt in 0..5 {
            let response = {
                let _guard = ensembl_request_gate().lock().await;
                // Ensembl has strict rate limits; keep requests serialized and paced.
                sleep(Duration::from_millis(250)).await;
                self.client.get(url).send().await?
            };

            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let snippet = truncate_for_error(&text, 4096);

            if status.as_u16() == 429 || status.as_u16() == 503 {
                last_retryable_error = Some(format!(
                    "Ensembl retryable status {status} for URL '{url}' on attempt {}. Response body: {snippet}",
                    attempt + 1
                ));
                sleep(Duration::from_secs(1 + attempt)).await;
                continue;
            }

            if !status.is_success() {
                return Err(anyhow!(
                    "Ensembl request failed for URL '{url}' with status {status}. Response body: {snippet}"
                ));
            }

            println!("{}", text);

            return serde_json::from_str::<T>(&text).map_err(|err| {
                anyhow!(
                    "Failed to deserialize Ensembl JSON response from URL '{url}': {err}. Response body: {snippet}"
                )
            });
        }

        Err(anyhow!(
            "{}",
            last_retryable_error
                .unwrap_or_else(|| format!("Ensembl request retries exhausted for URL '{url}'"))
        ))
    }

    async fn fetch_text_with_retries(&self, url: &str) -> Result<String> {
        let mut last_retryable_error = None;

        for attempt in 0..5 {
            let response = {
                let _guard = ensembl_request_gate().lock().await;
                sleep(Duration::from_millis(250)).await;
                self.client.get(url).send().await?
            };

            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            let snippet = truncate_for_error(&text, 4096);

            if status.as_u16() == 429 || status.as_u16() == 503 {
                last_retryable_error = Some(format!(
                    "Ensembl retryable status {status} for URL '{url}' on attempt {}. Response body: {snippet}",
                    attempt + 1
                ));
                sleep(Duration::from_secs(1 + attempt)).await;
                continue;
            }

            if !status.is_success() {
                return Err(anyhow!(
                    "Ensembl request failed for URL '{url}' with status {status}. Response body: {snippet}"
                ));
            }

            return Ok(text);
        }

        Err(anyhow!(
            "{}",
            last_retryable_error
                .unwrap_or_else(|| format!("Ensembl request retries exhausted for URL '{url}'"))
        ))
    }
}

fn ensembl_request_gate() -> &'static Mutex<()> {
    static REQUEST_GATE: OnceLock<Mutex<()>> = OnceLock::new();
    REQUEST_GATE.get_or_init(|| Mutex::new(()))
}

fn truncate_for_error(text: &str, max_len: usize) -> String {
    let cutoff = text
        .char_indices()
        .nth(max_len)
        .map(|(idx, _)| idx)
        .unwrap_or_else(|| text.len());

    if cutoff == text.len() {
        return text.to_owned();
    }

    let snippet = &text[..cutoff];
    format!("{snippet}...[truncated]")
}

#[derive(Debug, Deserialize)]
pub struct EnsemblGene {
    pub id: String,
    pub display_name: String,
    pub species: String,
    #[serde(default)]
    pub biotype: String,
    #[serde(default)]
    pub assembly_name: String,
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub start: u64,
    #[serde(default)]
    pub end: u64,
    #[serde(default)]
    pub strand: u64,
}

#[derive(Debug, Deserialize)]
pub struct EnsemblTranscript {
    pub id: String,
    pub display_name: Option<String>,
    pub species: String,
    #[serde(default)]
    pub biotype: String,
    #[serde(default, rename = "Parent")]
    pub gene_id: String,
    #[serde(default)]
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct EnsemblProtein {
    pub id: String,
    pub species: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub version: i64,
    #[serde(default)]
    pub biotype: String,
}

#[derive(Debug, Deserialize)]
pub struct EnsemblVariation {
    pub name: String,
    #[serde(default)]
    pub species: String,
    #[serde(default)]
    pub most_severe_consequence: String,
    #[serde(default)]
    pub minor_allele_freq: Option<f64>,
    #[serde(default, rename = "mappings")]
    pub mapping_results: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct EnsemblAssemblyInfo {
    pub assembly_name: String,
    pub assembly_accession: String,
    #[serde(default)]
    pub species: String,
    #[serde(default)]
    pub karyotype: Vec<String>,
    #[serde(default)]
    pub default_coord_system_version: String,
}
