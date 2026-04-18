use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use clap::{Args, Parser, Subcommand};
use flate2::read::GzDecoder;
use rand::seq::SliceRandom;
use rand::thread_rng;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tokio::time::sleep;

use cyto_vendor_examples::rcsb::RcsbClient;

const USER_AGENT: &str = "cyto-vendor-examples/0.1 (dump-cli)";
const NCBI_EUTILS_BASE: &str = "https://eutils.ncbi.nlm.nih.gov/entrez/eutils";
const ENSEMBL_BASE: &str = "https://rest.ensembl.org";
const GNOMAD_API_URL: &str = "https://gnomad.broadinstitute.org/api";
const GDC_API_BASE: &str = "https://api.gdc.cancer.gov";
const ENA_PORTAL_SEARCH_URL: &str = "https://www.ebi.ac.uk/ena/portal/api/search";
const CPTAC_API_BASE: &str = "https://services.cancerimagingarchive.net/nbia-api/services/v1";
const GTEX_PORTAL_V2_BASE_URL: &str = "https://gtexportal.org/api/v2";
const GTEX_HISTOLOGY_DZI_URL_ROOT: &str = "https://gtexportal.org/openslide/gtexhip/";
const GENCODE_BASE: &str = "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human";
const GENCODE_RELEASE: &str = "release_44";
const GENCODE_VERSION: &str = "44";
const GO_OBO_URLS: &[&str] = &[
    "https://ontologies.berkeleybop.org/go.obo",
    "https://purl.obolibrary.org/obo/go.obo",
];
const HPO_OBO_URLS: &[&str] = &[
    "https://ontologies.berkeleybop.org/hp.obo",
    "https://purl.obolibrary.org/obo/hp.obo",
];
const UCSC_CHAIN_URL: &str =
    "https://hgdownload.soe.ucsc.edu/goldenPath/hg19/liftOver/hg19ToHg38.over.chain.gz";
const UCSC_CYTOBAND_URL: &str =
    "https://hgdownload.soe.ucsc.edu/goldenPath/hg38/database/cytoBand.txt.gz";
const ENSEMBL_SYMBOLS: &[&str] = &[
    "BRCA1", "BRCA2", "TP53", "EGFR", "APOE", "MYC", "KRAS", "PTEN", "AKT1", "CFTR",
];
const DBSNP_RSIDS: &[&str] = &[
    "rs334",
    "rs699",
    "rs7412",
    "rs429358",
    "rs7903146",
    "rs1801133",
];
const RCSB_SEARCH_TERM: &str = "hemoglobin";
const CPTAC_COLLECTION: &str = "CPTAC-LSCC";

#[derive(Parser, Debug)]
#[command(author, version, about = "Vendor example dump CLI")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Dump(DumpArgs),
}

#[derive(Args, Debug)]
struct DumpArgs {
    #[command(subcommand)]
    entity: Option<DumpEntityCommand>,

    #[arg(
        short = 'o',
        long = "out",
        env = "OUT_DIR",
        default_value = "dump",
        global = true
    )]
    out_dir: PathBuf,

    #[arg(long, default_value_t = 1, global = true)]
    count: usize,

    #[arg(long, value_delimiter = ',', global = true)]
    vendor: Vec<String>,
}

#[derive(Subcommand, Debug, Clone)]
enum DumpEntityCommand {
    #[command(alias = "gene")]
    Genes,
    #[command(alias = "transcript")]
    Transcripts,
    #[command(alias = "protein")]
    Proteins,
    #[command(alias = "assembly")]
    Assemblies,
    #[command(alias = "variant")]
    Variants,
    #[command(alias = "structure")]
    Structures,
    #[command(alias = "entry")]
    Entries,
    #[command(alias = "run")]
    Runs,
    #[command(alias = "study")]
    Studies,
    #[command(alias = "sample")]
    Samples,
    #[command(alias = "nucleotide")]
    Nucleotides,
    #[command(alias = "publication")]
    Publications,
    #[command(alias = "taxonomy")]
    Taxonomies,
    #[command(alias = "biosample")]
    Biosamples,
    #[command(alias = "bioproject")]
    Bioprojects,
    #[command(alias = "mesh")]
    Meshes,
    #[command(alias = "ontology")]
    Ontologies,
    #[command(alias = "chain")]
    Chains,
    #[command(alias = "cytoband")]
    Cytobands,
    #[command(alias = "file")]
    Files,
    #[command(alias = "case")]
    Cases,
    #[command(alias = "collection")]
    Collections,
    Series,
    #[command(alias = "image")]
    Images,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum EntityKind {
    Genes,
    Transcripts,
    Proteins,
    Assemblies,
    Variants,
    Structures,
    Entries,
    Runs,
    Studies,
    Samples,
    Nucleotides,
    Publications,
    Taxonomies,
    Biosamples,
    Bioprojects,
    Meshes,
    Ontologies,
    Chains,
    Cytobands,
    Files,
    Cases,
    Collections,
    Series,
    Images,
}

impl DumpEntityCommand {
    fn kind(&self) -> EntityKind {
        match self {
            Self::Genes => EntityKind::Genes,
            Self::Transcripts => EntityKind::Transcripts,
            Self::Proteins => EntityKind::Proteins,
            Self::Assemblies => EntityKind::Assemblies,
            Self::Variants => EntityKind::Variants,
            Self::Structures => EntityKind::Structures,
            Self::Entries => EntityKind::Entries,
            Self::Runs => EntityKind::Runs,
            Self::Studies => EntityKind::Studies,
            Self::Samples => EntityKind::Samples,
            Self::Nucleotides => EntityKind::Nucleotides,
            Self::Publications => EntityKind::Publications,
            Self::Taxonomies => EntityKind::Taxonomies,
            Self::Biosamples => EntityKind::Biosamples,
            Self::Bioprojects => EntityKind::Bioprojects,
            Self::Meshes => EntityKind::Meshes,
            Self::Ontologies => EntityKind::Ontologies,
            Self::Chains => EntityKind::Chains,
            Self::Cytobands => EntityKind::Cytobands,
            Self::Files => EntityKind::Files,
            Self::Cases => EntityKind::Cases,
            Self::Collections => EntityKind::Collections,
            Self::Series => EntityKind::Series,
            Self::Images => EntityKind::Images,
        }
    }
}

impl EntityKind {
    fn dir_name(self) -> &'static str {
        match self {
            Self::Genes => "genes",
            Self::Transcripts => "transcripts",
            Self::Proteins => "proteins",
            Self::Assemblies => "assemblies",
            Self::Variants => "variants",
            Self::Structures => "structures",
            Self::Entries => "entries",
            Self::Runs => "runs",
            Self::Studies => "studies",
            Self::Samples => "samples",
            Self::Nucleotides => "nucleotides",
            Self::Publications => "publications",
            Self::Taxonomies => "taxonomies",
            Self::Biosamples => "biosamples",
            Self::Bioprojects => "bioprojects",
            Self::Meshes => "meshes",
            Self::Ontologies => "ontologies",
            Self::Chains => "chains",
            Self::Cytobands => "cytobands",
            Self::Files => "files",
            Self::Cases => "cases",
            Self::Collections => "collections",
            Self::Series => "series",
            Self::Images => "images",
        }
    }

    fn all() -> &'static [EntityKind] {
        &[
            Self::Genes,
            Self::Transcripts,
            Self::Proteins,
            Self::Assemblies,
            Self::Variants,
            Self::Structures,
            Self::Entries,
            Self::Runs,
            Self::Studies,
            Self::Samples,
            Self::Nucleotides,
            Self::Publications,
            Self::Taxonomies,
            Self::Biosamples,
            Self::Bioprojects,
            Self::Meshes,
            Self::Ontologies,
            Self::Chains,
            Self::Cytobands,
            Self::Files,
            Self::Cases,
            Self::Collections,
            Self::Series,
            Self::Images,
        ]
    }
}

struct DumpArtifact {
    vendor: &'static str,
    extension: String,
    bytes: Vec<u8>,
}

#[derive(Deserialize)]
struct NcbiSearchEnvelope {
    esearchresult: NcbiSearchResult,
}

#[derive(Deserialize)]
struct NcbiSearchResult {
    #[serde(default)]
    idlist: Vec<String>,
}

#[derive(Deserialize)]
struct UniProtSearchResponse {
    #[serde(default)]
    results: Vec<UniProtSearchItem>,
}

#[derive(Deserialize)]
struct UniProtSearchItem {
    #[serde(rename = "primaryAccession")]
    primary_accession: String,
}

#[derive(Deserialize)]
struct EnsemblSpeciesResponse {
    #[serde(default)]
    species: Vec<EnsemblSpecies>,
}

#[derive(Deserialize)]
struct EnsemblSpecies {
    name: String,
    #[serde(default)]
    groups: Vec<String>,
}

#[derive(Deserialize)]
struct GnomadGeneEnvelope {
    data: GnomadGeneData,
}

#[derive(Deserialize)]
struct GnomadGeneData {
    gene: GnomadGene,
}

#[derive(Deserialize)]
struct GnomadGene {
    #[serde(default)]
    variants: Vec<GnomadVariantRef>,
}

#[derive(Deserialize)]
struct GnomadVariantRef {
    variant_id: String,
}

#[derive(Deserialize)]
struct EnaAccessionRow {
    #[serde(default)]
    run_accession: String,
    #[serde(default)]
    study_accession: String,
    #[serde(default)]
    sample_accession: String,
}

#[derive(Deserialize)]
struct GdcHitsEnvelope {
    data: GdcHitsData,
}

#[derive(Deserialize)]
struct GdcHitsData {
    #[serde(default)]
    hits: Vec<Value>,
}

#[derive(Clone, Deserialize)]
struct CptacCollection {
    #[serde(rename = "Collection")]
    collection: String,
}

#[derive(Clone, Deserialize)]
struct CptacSeries {
    #[serde(rename = "SeriesInstanceUID")]
    series_instance_uid: String,
}

#[derive(Clone, Deserialize)]
struct CptacSopInstance {
    #[serde(rename = "SOPInstanceUID")]
    sop_instance_uid: String,
}

#[derive(Clone, Deserialize)]
struct GtexPaginatedResponse<T> {
    data: Vec<T>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GtexHistologySample {
    histology_image_id: String,
    subject_id: String,
}

pub async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Dump(args) => run_dump(args).await,
    }
}

async fn run_dump(args: DumpArgs) -> Result<()> {
    if args.count == 0 {
        bail!("--count must be greater than zero");
    }

    let selected_entities = match args.entity.as_ref() {
        Some(entity) => vec![entity.kind()],
        None => EntityKind::all().to_vec(),
    };

    let vendor_filter = normalize_vendor_filter(&args.vendor);
    validate_vendor_filter(&selected_entities, &vendor_filter)?;

    if let Some(entity) = args.entity.as_ref() {
        clear_path(&args.out_dir.join(entity.kind().dir_name()))?;
    } else {
        clear_path(&args.out_dir)?;
    }

    fs::create_dir_all(&args.out_dir).with_context(|| {
        format!(
            "Failed to create output directory '{}'",
            args.out_dir.display()
        )
    })?;

    let http = http_client()?;
    let rcsb = RcsbClient::new()?;

    for entity in selected_entities {
        println!("Dumping {}...", entity.dir_name());
        let artifacts = dump_entity(entity, args.count, &vendor_filter, &http, &rcsb).await?;
        write_artifacts(&args.out_dir, entity, artifacts)?;
    }

    Ok(())
}

async fn dump_entity(
    entity: EntityKind,
    count: usize,
    vendor_filter: &HashSet<String>,
    http: &Client,
    rcsb: &RcsbClient,
) -> Result<Vec<DumpArtifact>> {
    let mut artifacts = Vec::new();

    match entity {
        EntityKind::Genes => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids =
                    ncbi_search_ids(http, "gene", "Homo sapiens[Organism]", count * 20).await?;
                for gene_id in sample_unique(ids, count, "NCBI genes")? {
                    let value = ncbi_esummary(http, "gene", &gene_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "ensembl") {
                for payload in ensembl_gene_payloads(http, count).await? {
                    artifacts.push(json_artifact("ensembl", "json", payload)?);
                }
            }
            if vendor_allowed(vendor_filter, "gencode") {
                let gtf = fetch_gzip_text(http, &gencode_gtf_url()).await?;
                let records = parse_gtf_feature_records(&gtf, "gene");
                for record in sample_unique(records, count, "GENCODE genes")? {
                    artifacts.push(text_artifact("gencode", "gtf", record));
                }
            }
        }
        EntityKind::Transcripts => {
            if vendor_allowed(vendor_filter, "ensembl") {
                let ids = ensembl_transcript_ids(http, count * 10).await?;
                for transcript_id in sample_unique(ids, count, "Ensembl transcripts")? {
                    let value = fetch_json_get(
                        http,
                        &format!(
                            "{ENSEMBL_BASE}/lookup/id/{transcript_id}?content-type=application/json"
                        ),
                    )
                    .await?;
                    artifacts.push(json_artifact("ensembl", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "gencode") {
                let fasta = fetch_gzip_text(http, &gencode_transcript_fasta_url()).await?;
                let records = parse_fasta_records(&fasta);
                for record in sample_unique(records, count, "GENCODE transcripts")? {
                    artifacts.push(text_artifact("gencode", "fasta", record));
                }
            }
        }
        EntityKind::Proteins => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(
                    http,
                    "protein",
                    "Homo sapiens[Organism] AND srcdb_refseq[prop]",
                    count * 20,
                )
                .await?;
                for protein_id in sample_unique(ids, count, "NCBI proteins")? {
                    let value = ncbi_esummary(http, "protein", &protein_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "ensembl") {
                let ids = ensembl_protein_ids(http, count * 10).await?;
                for protein_id in sample_unique(ids, count, "Ensembl proteins")? {
                    let value = fetch_json_get(
                        http,
                        &format!(
                            "{ENSEMBL_BASE}/lookup/id/{protein_id}?content-type=application/json"
                        ),
                    )
                    .await?;
                    artifacts.push(json_artifact("ensembl", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "uniprot") {
                let accessions = uniprot_accessions(http, count * 20).await?;
                for accession in sample_unique(accessions, count, "UniProt proteins")? {
                    let value = fetch_json_get(
                        http,
                        &format!("https://rest.uniprot.org/uniprotkb/{accession}.json"),
                    )
                    .await?;
                    artifacts.push(json_artifact("uniprot", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "gencode") {
                let fasta = fetch_gzip_text(http, &gencode_protein_fasta_url()).await?;
                let records = parse_fasta_records(&fasta);
                for record in sample_unique(records, count, "GENCODE proteins")? {
                    artifacts.push(text_artifact("gencode", "fasta", record));
                }
            }
        }
        EntityKind::Assemblies => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids =
                    ncbi_search_ids(http, "assembly", "vertebrates[Organism]", count * 20).await?;
                for assembly_id in sample_unique(ids, count, "NCBI assemblies")? {
                    let value = ncbi_esummary(http, "assembly", &assembly_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "ensembl") {
                let species = ensembl_species(http, count * 20).await?;
                for species_name in sample_unique(species, count, "Ensembl assemblies")? {
                    let value = fetch_json_get(http, &format!(
                        "{ENSEMBL_BASE}/info/assembly/{species_name}?content-type=application/json"
                    ))
                    .await?;
                    artifacts.push(json_artifact("ensembl", "json", value)?);
                }
            }
        }
        EntityKind::Variants => {
            if vendor_allowed(vendor_filter, "ensembl") {
                for rsid in sample_unique(strings(DBSNP_RSIDS), count, "Ensembl variants")? {
                    let value = fetch_json_get(http, &format!(
                        "{ENSEMBL_BASE}/variation/homo_sapiens/{rsid}?content-type=application/json"
                    ))
                    .await?;
                    artifacts.push(json_artifact("ensembl", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "dbsnp") {
                for rsid in sample_unique(strings(DBSNP_RSIDS), count, "dbSNP variants")? {
                    let numeric_id = rsid.trim_start_matches("rs");
                    let value = fetch_json_get(
                        http,
                        &format!("https://api.ncbi.nlm.nih.gov/variation/v0/refsnp/{numeric_id}"),
                    )
                    .await?;
                    artifacts.push(json_artifact("dbsnp", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "dbvar") {
                let ids = ncbi_search_ids(http, "dbvar", "human[Organism]", count * 20).await?;
                for uid in sample_unique(ids, count, "dbVar variants")? {
                    let value = ncbi_esummary(http, "dbvar", &uid).await?;
                    artifacts.push(json_artifact("dbvar", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "gnomad") {
                let variant_ids = gnomad_variant_ids(http, count * 20).await?;
                for variant_id in sample_unique(variant_ids, count, "gnomAD variants")? {
                    let body = json!({
                        "query": r#"
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
"#,
                        "variables": { "variantId": variant_id },
                    });
                    let value = fetch_json_post(http, GNOMAD_API_URL, &body).await?;
                    artifacts.push(json_artifact("gnomad", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(http, "clinvar", "pathogenic", count * 20).await?;
                for clinvar_id in sample_unique(ids, count, "ClinVar variants")? {
                    let value = ncbi_esummary(http, "clinvar", &clinvar_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
        }
        EntityKind::Structures => {
            if vendor_allowed(vendor_filter, "rcsb") {
                let ids = rcsb
                    .search_entries_text(RCSB_SEARCH_TERM, 0, count * 20)
                    .await?
                    .items;
                for structure_id in sample_unique(ids, count, "RCSB structures")? {
                    let text = rcsb.download_cif_text(&structure_id).await?;
                    artifacts.push(text_artifact("rcsb", "cif", text));
                }
            }
        }
        EntityKind::Entries => {
            if vendor_allowed(vendor_filter, "rcsb") {
                let ids = rcsb
                    .search_entries_text(RCSB_SEARCH_TERM, 0, count * 20)
                    .await?
                    .items;
                for entry_id in sample_unique(ids, count, "RCSB entries")? {
                    let value = rcsb.fetch_entry_metadata_full(&entry_id).await?;
                    artifacts.push(json_artifact("rcsb", "json", value)?);
                }
            }
        }
        EntityKind::Runs => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(http, "sra", "human[Organism]", count * 20).await?;
                for sra_id in sample_unique(ids, count, "NCBI SRA runs")? {
                    let value = ncbi_esummary(http, "sra", &sra_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
            if vendor_allowed(vendor_filter, "ena") {
                let accessions =
                    ena_accessions(http, "read_run", "run_accession", count * 20).await?;
                for accession in sample_unique(accessions, count, "ENA runs")? {
                    let value = fetch_json_get(http, &format!(
                        "{ENA_PORTAL_SEARCH_URL}?result=read_run&query=run_accession=%22{accession}%22&fields=run_accession,experiment_accession,study_accession,sample_accession,library_strategy,read_count&format=json"
                    ))
                    .await?;
                    let record = first_json_array_record(value, &accession, "ENA run")?;
                    artifacts.push(json_artifact("ena", "json", record)?);
                }
            }
        }
        EntityKind::Studies => {
            if vendor_allowed(vendor_filter, "ena") {
                let accessions =
                    ena_accessions(http, "study", "study_accession", count * 20).await?;
                for accession in sample_unique(accessions, count, "ENA studies")? {
                    let value = fetch_json_get(http, &format!(
                        "{ENA_PORTAL_SEARCH_URL}?result=study&query=study_accession=%22{accession}%22&fields=study_accession,study_title&format=json"
                    ))
                    .await?;
                    let record = first_json_array_record(value, &accession, "ENA study")?;
                    artifacts.push(json_artifact("ena", "json", record)?);
                }
            }
        }
        EntityKind::Samples => {
            if vendor_allowed(vendor_filter, "ena") {
                let accessions =
                    ena_accessions(http, "sample", "sample_accession", count * 20).await?;
                for accession in sample_unique(accessions, count, "ENA samples")? {
                    let value = fetch_json_get(http, &format!(
                        "{ENA_PORTAL_SEARCH_URL}?result=sample&query=sample_accession=%22{accession}%22&fields=sample_accession,scientific_name,tax_id&format=json"
                    ))
                    .await?;
                    let record = first_json_array_record(value, &accession, "ENA sample")?;
                    artifacts.push(json_artifact("ena", "json", record)?);
                }
            }
        }
        EntityKind::Nucleotides => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(
                    http,
                    "nuccore",
                    "biomol mrna[prop] AND Homo sapiens[Organism]",
                    count * 20,
                )
                .await?;
                for nucleotide_id in sample_unique(ids, count, "NCBI nucleotides")? {
                    let value = ncbi_esummary(http, "nuccore", &nucleotide_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
        }
        EntityKind::Publications => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(http, "pubmed", "cancer", count * 20).await?;
                for pubmed_id in sample_unique(ids, count, "PubMed publications")? {
                    let value = ncbi_esummary(http, "pubmed", &pubmed_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
        }
        EntityKind::Taxonomies => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(http, "taxonomy", "species[Rank]", count * 20).await?;
                for taxid in sample_unique(ids, count, "NCBI taxonomies")? {
                    let value = ncbi_esummary(http, "taxonomy", &taxid).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
        }
        EntityKind::Biosamples => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(http, "biosample", "human", count * 20).await?;
                for biosample_id in sample_unique(ids, count, "NCBI biosamples")? {
                    let value = ncbi_esummary(http, "biosample", &biosample_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
        }
        EntityKind::Bioprojects => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(http, "bioproject", "human", count * 20).await?;
                for bioproject_id in sample_unique(ids, count, "NCBI bioprojects")? {
                    let value = ncbi_esummary(http, "bioproject", &bioproject_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
        }
        EntityKind::Meshes => {
            if vendor_allowed(vendor_filter, "ncbi") {
                let ids = ncbi_search_ids(http, "mesh", "neoplasms", count * 20).await?;
                for mesh_id in sample_unique(ids, count, "NCBI MeSH records")? {
                    let value = ncbi_esummary(http, "mesh", &mesh_id).await?;
                    artifacts.push(json_artifact("ncbi", "json", value)?);
                }
            }
        }
        EntityKind::Ontologies => {
            if vendor_allowed(vendor_filter, "go") {
                let obo = fetch_first_text(http, GO_OBO_URLS).await?;
                let terms = parse_obo_terms(&obo);
                for term in sample_unique(terms, count, "GO ontology terms")? {
                    artifacts.push(text_artifact("go", "obo", term));
                }
            }
            if vendor_allowed(vendor_filter, "hpo") {
                let obo = fetch_first_text(http, HPO_OBO_URLS).await?;
                let terms = parse_obo_terms(&obo);
                for term in sample_unique(terms, count, "HPO ontology terms")? {
                    artifacts.push(text_artifact("hpo", "obo", term));
                }
            }
        }
        EntityKind::Chains => {
            if vendor_allowed(vendor_filter, "ucsc") {
                let chain_text = fetch_gzip_text(http, UCSC_CHAIN_URL).await?;
                let blocks = parse_chain_blocks(&chain_text);
                for block in sample_unique(blocks, count, "UCSC chain blocks")? {
                    artifacts.push(text_artifact("ucsc", "chain", block));
                }
            }
        }
        EntityKind::Cytobands => {
            if vendor_allowed(vendor_filter, "ucsc") {
                let cytobands = fetch_gzip_text(http, UCSC_CYTOBAND_URL).await?;
                let rows = parse_non_comment_lines(&cytobands);
                for row in sample_unique(rows, count, "UCSC cytobands")? {
                    artifacts.push(text_artifact("ucsc", "txt", row));
                }
            }
        }
        EntityKind::Files => {
            if vendor_allowed(vendor_filter, "gdc") {
                let files = gdc_file_hits(http, count * 20).await?;
                for record in sample_unique(files, count, "GDC files")? {
                    artifacts.push(json_artifact("gdc", "json", record)?);
                }
            }
        }
        EntityKind::Cases => {
            if vendor_allowed(vendor_filter, "gdc") {
                let cases = gdc_case_hits(http, count * 20).await?;
                for record in sample_unique(cases, count, "GDC cases")? {
                    artifacts.push(json_artifact("gdc", "json", record)?);
                }
            }
        }
        EntityKind::Collections => {
            if vendor_allowed(vendor_filter, "cptac") {
                let collections = cptac_collection_names(http).await?;
                for collection in sample_unique(collections, count, "CPTAC collections")? {
                    artifacts.push(json_artifact(
                        "cptac",
                        "json",
                        json!({ "collection": collection }),
                    )?);
                }
            }
        }
        EntityKind::Series => {
            if vendor_allowed(vendor_filter, "cptac") {
                let series = cptac_series_hits(http).await?;
                for record in sample_unique(series, count, "CPTAC series")? {
                    artifacts.push(json_artifact("cptac", "json", record)?);
                }
            }
        }
        EntityKind::Images => {
            if vendor_allowed(vendor_filter, "cptac") {
                for bytes in cptac_image_bytes(http, count).await? {
                    artifacts.push(binary_artifact("cptac", "dcm", bytes));
                }
            }
            if vendor_allowed(vendor_filter, "gtex") {
                for (extension, bytes) in gtex_image_bytes(http, count).await? {
                    artifacts.push(binary_artifact("gtex", &extension, bytes));
                }
            }
        }
    }

    Ok(artifacts)
}

fn write_artifacts(out_dir: &Path, entity: EntityKind, artifacts: Vec<DumpArtifact>) -> Result<()> {
    let entity_dir = out_dir.join(entity.dir_name());
    fs::create_dir_all(&entity_dir).with_context(|| {
        format!(
            "Failed to create entity directory '{}'",
            entity_dir.display()
        )
    })?;

    let mut vendor_seen = std::collections::HashMap::<&'static str, usize>::new();
    for artifact in artifacts {
        let index = vendor_seen.entry(artifact.vendor).or_insert(0);
        let file_name = format!(
            "{}_{}_{}.{}",
            artifact.vendor,
            entity.dir_name(),
            *index,
            artifact.extension
        );
        *index += 1;
        let path = entity_dir.join(file_name);
        fs::write(&path, artifact.bytes)
            .with_context(|| format!("Failed to write '{}'", path.display()))?;
    }

    Ok(())
}

fn normalize_vendor_filter(values: &[String]) -> HashSet<String> {
    values
        .iter()
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn validate_vendor_filter(entities: &[EntityKind], vendor_filter: &HashSet<String>) -> Result<()> {
    if vendor_filter.is_empty() {
        return Ok(());
    }

    let known: BTreeSet<String> = entities
        .iter()
        .flat_map(|entity| supported_vendors(*entity))
        .map(|vendor| vendor.to_string())
        .collect();

    let unknown: Vec<_> = vendor_filter
        .iter()
        .filter(|vendor| !known.contains(*vendor))
        .cloned()
        .collect();

    if unknown.is_empty() {
        return Ok(());
    }

    bail!(
        "Unknown vendor filter(s): {}. Supported vendors for this selection: {}",
        unknown.join(", "),
        known.into_iter().collect::<Vec<_>>().join(", ")
    )
}

fn supported_vendors(entity: EntityKind) -> &'static [&'static str] {
    match entity {
        EntityKind::Genes => &["ensembl", "gencode", "ncbi"],
        EntityKind::Transcripts => &["ensembl", "gencode"],
        EntityKind::Proteins => &["ensembl", "gencode", "ncbi", "uniprot"],
        EntityKind::Assemblies => &["ensembl", "ncbi"],
        EntityKind::Variants => &["dbsnp", "dbvar", "ensembl", "gnomad", "ncbi"],
        EntityKind::Structures => &["rcsb"],
        EntityKind::Entries => &["rcsb"],
        EntityKind::Runs => &["ena", "ncbi"],
        EntityKind::Studies => &["ena"],
        EntityKind::Samples => &["ena"],
        EntityKind::Nucleotides => &["ncbi"],
        EntityKind::Publications => &["ncbi"],
        EntityKind::Taxonomies => &["ncbi"],
        EntityKind::Biosamples => &["ncbi"],
        EntityKind::Bioprojects => &["ncbi"],
        EntityKind::Meshes => &["ncbi"],
        EntityKind::Ontologies => &["go", "hpo"],
        EntityKind::Chains => &["ucsc"],
        EntityKind::Cytobands => &["ucsc"],
        EntityKind::Files => &["gdc"],
        EntityKind::Cases => &["gdc"],
        EntityKind::Collections => &["cptac"],
        EntityKind::Series => &["cptac"],
        EntityKind::Images => &["cptac", "gtex"],
    }
}

fn vendor_allowed(filter: &HashSet<String>, vendor: &str) -> bool {
    filter.is_empty() || filter.contains(vendor)
}

fn clear_path(path: &Path) -> Result<()> {
    if path.exists() {
        if path.is_dir() {
            fs::remove_dir_all(path)
                .with_context(|| format!("Failed to clear directory '{}'", path.display()))?;
        } else {
            fs::remove_file(path)
                .with_context(|| format!("Failed to remove file '{}'", path.display()))?;
        }
    }
    Ok(())
}

fn http_client() -> Result<Client> {
    Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(180))
        .build()
        .context("Failed to build HTTP client")
}

async fn fetch_text_get(client: &Client, url: &str) -> Result<String> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET '{}' failed", url))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "GET '{}' failed with status {}: {}",
            url,
            status,
            truncate(&text, 4096)
        );
    }
    Ok(text)
}

async fn fetch_bytes_get(client: &Client, url: &str) -> Result<Vec<u8>> {
    let response = client
        .get(url)
        .send()
        .await
        .with_context(|| format!("GET '{}' failed", url))?;
    let status = response.status();
    let bytes = response.bytes().await.unwrap_or_default().to_vec();
    if !status.is_success() {
        let body = String::from_utf8_lossy(&bytes);
        bail!(
            "GET '{}' failed with status {}: {}",
            url,
            status,
            truncate(&body, 4096)
        );
    }
    Ok(bytes)
}

async fn fetch_json_get(client: &Client, url: &str) -> Result<Value> {
    let text = fetch_text_get(client, url).await?;
    serde_json::from_str(&text).with_context(|| format!("Failed to parse JSON from '{}'", url))
}

async fn fetch_json_post(client: &Client, url: &str, body: &Value) -> Result<Value> {
    let response = client
        .post(url)
        .json(body)
        .send()
        .await
        .with_context(|| format!("POST '{}' failed", url))?;
    let status = response.status();
    let text = response.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!(
            "POST '{}' failed with status {}: {}",
            url,
            status,
            truncate(&text, 4096)
        );
    }
    serde_json::from_str(&text).with_context(|| format!("Failed to parse JSON from '{}'", url))
}

async fn fetch_json_typed_get<T>(client: &Client, url: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let text = fetch_text_get(client, url).await?;
    serde_json::from_str(&text).with_context(|| format!("Failed to parse JSON from '{}'", url))
}

async fn fetch_first_text(client: &Client, urls: &[&str]) -> Result<String> {
    let mut last_error = None;
    for url in urls {
        match fetch_text_get(client, url).await {
            Ok(text) => return Ok(text),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow!("No URLs provided")))
}

async fn fetch_gzip_text(client: &Client, url: &str) -> Result<String> {
    let bytes = fetch_bytes_get(client, url).await?;
    let mut decoder = GzDecoder::new(&bytes[..]);
    let mut out = String::new();
    decoder.read_to_string(&mut out)?;
    Ok(out)
}

fn truncate(text: &str, max_len: usize) -> String {
    if text.chars().count() <= max_len {
        return text.to_string();
    }

    let cutoff = text
        .char_indices()
        .nth(max_len)
        .map(|(idx, _)| idx)
        .unwrap_or(text.len());
    format!("{}...[truncated]", &text[..cutoff])
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn sample_unique<T: Clone>(mut values: Vec<T>, count: usize, label: &str) -> Result<Vec<T>> {
    if values.len() < count {
        bail!(
            "Requested {} unique {} but only {} are available",
            count,
            label,
            values.len()
        );
    }

    values.shuffle(&mut thread_rng());
    values.truncate(count);
    Ok(values)
}

fn dedupe_strings(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

fn json_artifact(vendor: &'static str, extension: &str, value: Value) -> Result<DumpArtifact> {
    Ok(DumpArtifact {
        vendor,
        extension: extension.to_string(),
        bytes: serde_json::to_vec_pretty(&value)?,
    })
}

fn text_artifact(vendor: &'static str, extension: &str, text: String) -> DumpArtifact {
    DumpArtifact {
        vendor,
        extension: extension.to_string(),
        bytes: ensure_trailing_newline(text).into_bytes(),
    }
}

fn binary_artifact(vendor: &'static str, extension: &str, bytes: Vec<u8>) -> DumpArtifact {
    DumpArtifact {
        vendor,
        extension: extension.to_string(),
        bytes,
    }
}

fn ensure_trailing_newline(mut text: String) -> String {
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

async fn ncbi_search_ids(
    client: &Client,
    db: &str,
    term: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let mut last_rate_limit = None;
    for attempt in 0..5 {
        let response = {
            let _guard = ncbi_request_gate().lock().await;
            sleep(Duration::from_millis(350)).await;
            client
                .get(format!("{NCBI_EUTILS_BASE}/esearch.fcgi"))
                .query(&[
                    ("db", db),
                    ("term", term),
                    ("retmode", "json"),
                    ("retmax", &limit.to_string()),
                ])
                .send()
                .await?
        };
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if status.as_u16() == 429 {
            last_rate_limit = Some(format!(
                "NCBI esearch rate-limited for db '{}' term '{}' on attempt {}: {}",
                db,
                term,
                attempt + 1,
                truncate(&text, 1024)
            ));
            sleep(Duration::from_secs(1 + attempt as u64)).await;
            continue;
        }
        if !status.is_success() {
            bail!(
                "NCBI esearch failed for db '{}' term '{}': {} {}",
                db,
                term,
                status,
                truncate(&text, 4096)
            );
        }
        let payload: NcbiSearchEnvelope = serde_json::from_str(&text).with_context(|| {
            format!(
                "Failed to parse NCBI esearch response for db '{}' term '{}'",
                db, term
            )
        })?;
        let ids = dedupe_strings(payload.esearchresult.idlist);
        if ids.is_empty() {
            bail!(
                "NCBI esearch returned no ids for db '{}' term '{}'",
                db,
                term
            );
        }
        return Ok(ids);
    }

    Err(anyhow!(last_rate_limit.unwrap_or_else(|| {
        format!(
            "NCBI esearch retries exhausted for db '{}' term '{}'",
            db, term
        )
    })))
}

async fn ncbi_esummary(client: &Client, db: &str, id: &str) -> Result<Value> {
    let mut last_rate_limit = None;
    for attempt in 0..5 {
        let response = {
            let _guard = ncbi_request_gate().lock().await;
            sleep(Duration::from_millis(350)).await;
            client
                .get(format!("{NCBI_EUTILS_BASE}/esummary.fcgi"))
                .query(&[("db", db), ("id", id), ("retmode", "json")])
                .send()
                .await?
        };
        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if status.as_u16() == 429 {
            last_rate_limit = Some(format!(
                "NCBI esummary rate-limited for db '{}' id '{}' on attempt {}: {}",
                db,
                id,
                attempt + 1,
                truncate(&text, 1024)
            ));
            sleep(Duration::from_secs(1 + attempt as u64)).await;
            continue;
        }
        if !status.is_success() {
            bail!(
                "NCBI esummary failed for db '{}' id '{}': {} {}",
                db,
                id,
                status,
                truncate(&text, 4096)
            );
        }
        return serde_json::from_str(&text).with_context(|| {
            format!(
                "Failed to parse NCBI esummary response for db '{}' id '{}'",
                db, id
            )
        });
    }

    Err(anyhow!(last_rate_limit.unwrap_or_else(|| {
        format!(
            "NCBI esummary retries exhausted for db '{}' id '{}'",
            db, id
        )
    })))
}

fn ncbi_request_gate() -> &'static Mutex<()> {
    static REQUEST_GATE: OnceLock<Mutex<()>> = OnceLock::new();
    REQUEST_GATE.get_or_init(|| Mutex::new(()))
}

async fn ensembl_gene_payloads(client: &Client, count: usize) -> Result<Vec<Value>> {
    let symbols = sample_unique(strings(ENSEMBL_SYMBOLS), count, "Ensembl gene symbols")?;
    let mut payloads = Vec::new();
    for symbol in symbols {
        payloads.push(
            fetch_json_get(
                client,
                &format!(
                    "{ENSEMBL_BASE}/lookup/symbol/homo_sapiens/{symbol}?expand=1;content-type=application/json"
                ),
            )
            .await?,
        );
    }
    Ok(payloads)
}

async fn ensembl_symbol_payloads(client: &Client) -> Result<Vec<Value>> {
    let mut payloads = Vec::new();
    for symbol in ENSEMBL_SYMBOLS {
        let payload = fetch_json_get(
            client,
            &format!(
                "{ENSEMBL_BASE}/lookup/symbol/homo_sapiens/{symbol}?expand=1;content-type=application/json"
            ),
        )
        .await?;
        payloads.push(payload);
    }
    Ok(payloads)
}

async fn ensembl_transcript_ids(client: &Client, _limit: usize) -> Result<Vec<String>> {
    let payloads = ensembl_symbol_payloads(client).await?;
    let mut ids = Vec::new();
    for payload in payloads {
        if let Some(transcripts) = payload.get("Transcript").and_then(Value::as_array) {
            for transcript in transcripts {
                if let Some(id) = transcript.get("id").and_then(Value::as_str) {
                    ids.push(id.to_string());
                }
            }
        }
    }
    Ok(dedupe_strings(ids))
}

async fn ensembl_protein_ids(client: &Client, _limit: usize) -> Result<Vec<String>> {
    let payloads = ensembl_symbol_payloads(client).await?;
    let mut ids = Vec::new();
    for payload in payloads {
        if let Some(transcripts) = payload.get("Transcript").and_then(Value::as_array) {
            for transcript in transcripts {
                if let Some(id) = transcript
                    .get("Translation")
                    .and_then(|translation| translation.get("id"))
                    .and_then(Value::as_str)
                {
                    ids.push(id.to_string());
                }
            }
        }
    }
    Ok(dedupe_strings(ids))
}

async fn ensembl_species(client: &Client, _limit: usize) -> Result<Vec<String>> {
    let payload: EnsemblSpeciesResponse = fetch_json_typed_get(
        client,
        &format!("{ENSEMBL_BASE}/info/species?content-type=application/json"),
    )
    .await?;
    let species = payload
        .species
        .into_iter()
        .filter(|entry| entry.groups.iter().any(|group| group == "core"))
        .map(|entry| entry.name)
        .collect::<Vec<_>>();
    let species = dedupe_strings(species);
    if species.is_empty() {
        bail!("Ensembl species listing returned no species");
    }
    Ok(species)
}

async fn uniprot_accessions(client: &Client, limit: usize) -> Result<Vec<String>> {
    let payload: UniProtSearchResponse = fetch_json_typed_get(
        client,
        &format!(
            "https://rest.uniprot.org/uniprotkb/search?query=reviewed:true+AND+organism_id:9606&fields=accession&format=json&size={limit}"
        ),
    )
    .await?;
    let accessions = payload
        .results
        .into_iter()
        .map(|item| item.primary_accession)
        .collect::<Vec<_>>();
    let accessions = dedupe_strings(accessions);
    if accessions.is_empty() {
        bail!("UniProt search returned no accessions");
    }
    Ok(accessions)
}

async fn gnomad_variant_ids(client: &Client, _limit: usize) -> Result<Vec<String>> {
    let body = json!({
        "query": r#"
query {
    gene(gene_id: "ENSG00000139618", reference_genome: GRCh38) {
    variants(dataset: gnomad_r4) {
      variant_id
    }
  }
}
"#,
    });
    let payload: GnomadGeneEnvelope =
        serde_json::from_value(fetch_json_post(client, GNOMAD_API_URL, &body).await?)
            .context("Failed to parse gnomAD gene variant listing")?;
    let ids = payload
        .data
        .gene
        .variants
        .into_iter()
        .map(|variant| variant.variant_id)
        .collect::<Vec<_>>();
    let ids = dedupe_strings(ids);
    if ids.is_empty() {
        bail!("gnomAD returned no variant ids");
    }
    Ok(ids)
}

async fn ena_accessions(
    client: &Client,
    result: &str,
    field: &str,
    limit: usize,
) -> Result<Vec<String>> {
    let url =
        format!("{ENA_PORTAL_SEARCH_URL}?result={result}&fields={field}&limit={limit}&format=json");
    let payload: Vec<EnaAccessionRow> = fetch_json_typed_get(client, &url).await?;
    let values = payload
        .into_iter()
        .filter_map(|row| match field {
            "run_accession" if !row.run_accession.is_empty() => Some(row.run_accession),
            "study_accession" if !row.study_accession.is_empty() => Some(row.study_accession),
            "sample_accession" if !row.sample_accession.is_empty() => Some(row.sample_accession),
            _ => None,
        })
        .collect::<Vec<_>>();
    let values = dedupe_strings(values);
    if values.is_empty() {
        bail!(
            "ENA listing returned no values for result '{}' field '{}'",
            result,
            field
        );
    }
    Ok(values)
}

async fn gdc_file_hits(client: &Client, limit: usize) -> Result<Vec<Value>> {
    let body = json!({
        "filters": {
            "op": "and",
            "content": [
                {
                    "op": "in",
                    "content": {
                        "field": "cases.project.project_id",
                        "value": ["TCGA-BRCA"]
                    }
                },
                {
                    "op": "in",
                    "content": {
                        "field": "data_format",
                        "value": ["SVS"]
                    }
                }
            ]
        },
        "fields": "file_id,file_name,data_category,data_type,data_format,access,cases.submitter_id,cases.project.project_id,cases.samples.submitter_id",
        "format": "JSON",
        "size": limit
    });
    let payload: GdcHitsEnvelope = serde_json::from_value(
        fetch_json_post(client, &format!("{GDC_API_BASE}/files"), &body).await?,
    )
    .context("Failed to parse GDC file hits")?;
    let hits = payload.data.hits;
    if hits.is_empty() {
        bail!("GDC file query returned no hits");
    }
    Ok(hits)
}

async fn gdc_case_hits(client: &Client, limit: usize) -> Result<Vec<Value>> {
    let body = json!({
        "filters": {
            "op": "in",
            "content": {
                "field": "project.project_id",
                "value": ["TCGA-BRCA"]
            }
        },
        "fields": "submitter_id,submitter_sample_ids,project.project_id,files.file_name,files.data_category,files.data_type,files.data_format,files.file_id",
        "expand": "project,files",
        "format": "JSON",
        "size": limit
    });
    let payload: GdcHitsEnvelope = serde_json::from_value(
        fetch_json_post(client, &format!("{GDC_API_BASE}/cases"), &body).await?,
    )
    .context("Failed to parse GDC case hits")?;
    let hits = payload.data.hits;
    if hits.is_empty() {
        bail!("GDC case query returned no hits");
    }
    Ok(hits)
}

async fn cptac_collection_names(client: &Client) -> Result<Vec<String>> {
    let collections: Vec<CptacCollection> = fetch_json_typed_get(
        client,
        &format!("{CPTAC_API_BASE}/getCollectionValues?format=json"),
    )
    .await?;
    let names = collections
        .into_iter()
        .map(|collection| collection.collection)
        .collect::<Vec<_>>();
    let names = dedupe_strings(names);
    if names.is_empty() {
        bail!("CPTAC collection listing returned no collections");
    }
    Ok(names)
}

async fn cptac_series_hits(client: &Client) -> Result<Vec<Value>> {
    let hits: Vec<Value> = fetch_json_typed_get(
        client,
        &format!("{CPTAC_API_BASE}/getSeries?Collection={CPTAC_COLLECTION}&format=json"),
    )
    .await?;
    if hits.is_empty() {
        bail!("CPTAC series listing returned no hits");
    }
    Ok(hits)
}

async fn cptac_image_bytes(client: &Client, count: usize) -> Result<Vec<Vec<u8>>> {
    let series: Vec<CptacSeries> = fetch_json_typed_get(
        client,
        &format!("{CPTAC_API_BASE}/getSeries?Collection={CPTAC_COLLECTION}&format=json"),
    )
    .await?;
    let series = sample_unique(series, count, "CPTAC images")?;
    let mut images = Vec::new();
    for series_entry in series {
        let sops: Vec<CptacSopInstance> = fetch_json_typed_get(
            client,
            &format!(
                "{CPTAC_API_BASE}/getSOPInstanceUIDs?SeriesInstanceUID={}&format=json",
                series_entry.series_instance_uid
            ),
        )
        .await?;
        let sop = sops.into_iter().next().ok_or_else(|| {
            anyhow!(
                "CPTAC series '{}' returned no SOP instance UIDs",
                series_entry.series_instance_uid
            )
        })?;
        let bytes = fetch_bytes_get(
            client,
            &format!(
                "{CPTAC_API_BASE}/getSingleImage?SeriesInstanceUID={}&SOPInstanceUID={}",
                series_entry.series_instance_uid, sop.sop_instance_uid
            ),
        )
        .await?;
        images.push(bytes);
    }
    Ok(images)
}

async fn gtex_image_bytes(client: &Client, count: usize) -> Result<Vec<(String, Vec<u8>)>> {
    let payload: GtexPaginatedResponse<GtexHistologySample> = fetch_json_typed_get(
        client,
        &format!(
            "{GTEX_PORTAL_V2_BASE_URL}/histology/image?itemsPerPage={}&page=0",
            count * 20
        ),
    )
    .await?;
    let samples = sample_unique(payload.data, count, "GTEx histology samples")?;
    let mut images = Vec::new();
    for sample in samples {
        let dzi_url = format!(
            "{GTEX_HISTOLOGY_DZI_URL_ROOT}{}/{}.dzi",
            sample.subject_id, sample.histology_image_id
        );
        let dzi_xml = fetch_text_get(client, &dzi_url).await?;
        let extension = extract_dzi_format(&dzi_xml)?;
        let tile_url = format!(
            "{GTEX_HISTOLOGY_DZI_URL_ROOT}{}/{}_files/0/0_0.{}",
            sample.subject_id, sample.histology_image_id, extension
        );
        let bytes = fetch_bytes_get(client, &tile_url).await?;
        images.push((extension, bytes));
    }
    Ok(images)
}

fn first_json_array_record(value: Value, id: &str, kind: &str) -> Result<Value> {
    match value {
        Value::Array(mut items) => items
            .drain(..)
            .next()
            .ok_or_else(|| anyhow!("{} response for '{}' contained no records", kind, id)),
        other => Ok(other),
    }
}

fn parse_gtf_feature_records(gtf: &str, feature: &str) -> Vec<String> {
    gtf.lines()
        .filter(|line| !line.starts_with('#'))
        .filter(|line| line.split('\t').nth(2) == Some(feature))
        .map(|line| line.to_string())
        .collect()
}

fn parse_fasta_records(fasta: &str) -> Vec<String> {
    let mut records = Vec::new();
    let mut current = Vec::new();
    for line in fasta.lines() {
        if line.starts_with('>') && !current.is_empty() {
            records.push(current.join("\n"));
            current.clear();
        }
        current.push(line.to_string());
    }
    if !current.is_empty() {
        records.push(current.join("\n"));
    }
    records
}

fn parse_obo_terms(obo: &str) -> Vec<String> {
    let header = obo
        .lines()
        .take_while(|line| !line.trim_start().starts_with("[Term]"))
        .collect::<Vec<_>>()
        .join("\n");
    obo.split("\n[Term]")
        .skip(1)
        .map(|block| format!("{}\n[Term]{}\n", header.trim_end(), block.trim_end()))
        .collect()
}

fn parse_chain_blocks(chain_text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current = Vec::new();
    for line in chain_text.lines() {
        if line.starts_with("chain ") && !current.is_empty() {
            blocks.push(current.join("\n"));
            current.clear();
        }
        current.push(line.to_string());
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

fn parse_non_comment_lines(text: &str) -> Vec<String> {
    text.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

fn extract_dzi_format(dzi_xml: &str) -> Result<String> {
    let marker = "Format=\"";
    let start = dzi_xml
        .find(marker)
        .ok_or_else(|| anyhow!("DeepZoom descriptor missing Format attribute"))?
        + marker.len();
    let rest = &dzi_xml[start..];
    let end = rest
        .find('"')
        .ok_or_else(|| anyhow!("DeepZoom descriptor contains an unterminated Format attribute"))?;
    let format = rest[..end].trim().to_ascii_lowercase();
    if format.is_empty() {
        bail!("DeepZoom descriptor contained an empty Format attribute");
    }
    Ok(format)
}

fn gencode_gtf_url() -> String {
    format!(
        "{}/{}/gencode.v{}.annotation.gtf.gz",
        GENCODE_BASE, GENCODE_RELEASE, GENCODE_VERSION
    )
}

fn gencode_transcript_fasta_url() -> String {
    format!(
        "{}/{}/gencode.v{}.transcripts.fa.gz",
        GENCODE_BASE, GENCODE_RELEASE, GENCODE_VERSION
    )
}

fn gencode_protein_fasta_url() -> String {
    format!(
        "{}/{}/gencode.v{}.pc_translations.fa.gz",
        GENCODE_BASE, GENCODE_RELEASE, GENCODE_VERSION
    )
}
