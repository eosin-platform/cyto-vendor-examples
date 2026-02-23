use flate2::read::GzDecoder;
use reqwest::Client;
use std::io::Read;
use std::time::Duration;
use tokio::sync::OnceCell;

type AnyError = Box<dyn std::error::Error + Send + Sync>;

const GENCODE_BASE: &str = "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human";
const GENCODE_RELEASE: &str = "release_44";
const GENCODE_VERSION: &str = "44";
static GENCODE_GTF_CACHE: OnceCell<String> = OnceCell::const_new();

fn gencode_client() -> Result<Client, AnyError> {
    let client = Client::builder()
        .user_agent("cyto-vendor-examples/0.1 (gencode-integration-test)")
        .timeout(Duration::from_secs(30)) // may need to tune for CI
        .build()?;
    Ok(client)
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

async fn fetch_and_gunzip_to_string(client: &Client, url: &str) -> Result<String, AnyError> {
    let response = client.get(url).send().await?;
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(
            format!("GENCODE request failed for URL '{url}' with status {status}: {body}").into(),
        );
    }

    let bytes = response.bytes().await?;
    let mut decoder = GzDecoder::new(&bytes[..]);
    let mut out = String::new();
    decoder.read_to_string(&mut out)?;
    Ok(out)
}

async fn cached_gencode_gtf(client: &Client) -> Result<&'static String, AnyError> {
    GENCODE_GTF_CACHE
        .get_or_try_init(|| async {
            let url = gencode_gtf_url();
            fetch_and_gunzip_to_string(client, &url).await
        })
        .await
}

#[derive(Debug)]
struct GtfRecord<'a> {
    seqname: &'a str,
    source: &'a str,
    feature: &'a str,
    start: u64,
    end: u64,
    score: &'a str,
    strand: char,
    frame: &'a str,
    attributes: &'a str,
}

impl<'a> GtfRecord<'a> {
    fn parse(line: &'a str) -> Option<GtfRecord<'a>> {
        if line.starts_with('#') {
            return None;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 9 {
            return None;
        }

        let start = fields[3].parse().ok()?;
        let end = fields[4].parse().ok()?;
        let strand_ch = fields[6].chars().next().unwrap_or('.');

        Some(GtfRecord {
            seqname: fields[0],
            source: fields[1],
            feature: fields[2],
            start,
            end,
            score: fields[5],
            strand: strand_ch,
            frame: fields[7],
            attributes: fields[8],
        })
    }

    fn attr(&self, key: &str) -> Option<String> {
        let pattern = format!(r#"{key} ""#);
        let idx = self.attributes.find(&pattern)?;
        let after = &self.attributes[idx + pattern.len()..];
        let end_quote = after.find('"')?;
        Some(after[..end_quote].to_string())
    }

    fn gene_id(&self) -> Option<String> {
        self.attr("gene_id")
    }

    fn transcript_id(&self) -> Option<String> {
        self.attr("transcript_id")
    }

    fn gene_name(&self) -> Option<String> {
        self.attr("gene_name")
    }

    fn gene_type(&self) -> Option<String> {
        self.attr("gene_type")
    }
}

fn parse_fasta_sequences(fasta: &str) -> Vec<(String, String)> {
    let mut result = Vec::new();
    let mut current_id: Option<String> = None;
    let mut current_seq = String::new();

    for line in fasta.lines() {
        if line.starts_with('>') {
            if let Some(id) = current_id.take() {
                result.push((id, current_seq.clone()));
                current_seq.clear();
            }
            let header = line.trim_start_matches('>');
            let first_token = header.split_whitespace().next().unwrap_or("");
            current_id = Some(first_token.to_string());
        } else {
            current_seq.push_str(line.trim());
        }
    }

    if let Some(id) = current_id {
        result.push((id, current_seq));
    }

    result
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
async fn gencode_gtf_contains_brca2_gene() -> Result<(), AnyError> {
    let client = gencode_client()?;
    let gtf = cached_gencode_gtf(&client).await?;

    assert!(
        gtf.lines().any(|line| {
            line.starts_with("#!genome-build")
                || (line.starts_with("##") && line.to_ascii_lowercase().contains("gencode"))
        }),
        "Expected GENCODE GTF to contain '#!genome-build' or a '##' header mentioning GENCODE"
    );

    let mut found_brca2 = false;

    for line in gtf.lines() {
        if let Some(rec) = GtfRecord::parse(line)
            && rec.feature == "gene"
            && let (Some(gene_id), Some(gene_name)) = (rec.gene_id(), rec.gene_name())
            && ({
                let gene_id_base = gene_id
                    .split_once('.')
                    .map(|(base, _)| base)
                    .unwrap_or(gene_id.as_str());
                gene_id_base == "ENSG00000139618" || gene_name.eq_ignore_ascii_case("BRCA2")
            })
        {
            found_brca2 = true;
            assert_eq!(
                rec.seqname, "chr13",
                "Expected BRCA2 on chr13 in GENCODE GTF"
            );
            assert!(
                !rec.source.trim().is_empty(),
                "Expected non-empty GTF source for BRCA2"
            );
            assert!(rec.start < rec.end, "Expected BRCA2 gene start < end");
            assert!(
                matches!(rec.strand, '+' | '-'),
                "Expected valid strand for BRCA2 gene"
            );
            if let Some(gtype) = rec.gene_type() {
                assert!(
                    gtype.to_ascii_lowercase().contains("protein_coding"),
                    "Expected BRCA2 gene_type to contain 'protein_coding', got '{gtype}'"
                );
            }
            let _ = rec.score;
            let _ = rec.frame;
            break;
        }
    }

    assert!(
        found_brca2,
        "Expected GENCODE GTF to contain a gene record for BRCA2 (ENSG00000139618)"
    );

    Ok(())
}

#[tokio::test]
async fn gencode_gtf_contains_brca2_transcript() -> Result<(), AnyError> {
    let client = gencode_client()?;
    let gtf = cached_gencode_gtf(&client).await?;

    let mut found_tx = false;

    for line in gtf.lines() {
        if let Some(rec) = GtfRecord::parse(line)
            && rec.feature == "transcript"
        {
            let gene_id = rec.gene_id().unwrap_or_default();
            let transcript_id = rec.transcript_id().unwrap_or_default();
            let gene_id_base = gene_id
                .split_once('.')
                .map(|(base, _)| base)
                .unwrap_or(gene_id.as_str());
            let transcript_id_base = transcript_id
                .split_once('.')
                .map(|(base, _)| base)
                .unwrap_or(transcript_id.as_str());
            if gene_id_base == "ENSG00000139618"
                && (transcript_id.starts_with("ENST00000380152")
                    || transcript_id_base == "ENST00000380152")
            {
                found_tx = true;
                assert_eq!(
                    rec.seqname, "chr13",
                    "Expected BRCA2 transcript ENST00000380152* on chr13"
                );
                assert!(
                    !rec.source.trim().is_empty(),
                    "Expected non-empty GTF source for BRCA2 transcript"
                );
                assert!(rec.start < rec.end, "Expected transcript start < end");
                assert!(
                    matches!(rec.strand, '+' | '-'),
                    "Expected valid strand for BRCA2 transcript"
                );
                let _ = rec.score;
                let _ = rec.frame;
                break;
            }
        }
    }

    assert!(
        found_tx,
        "Expected GENCODE GTF to contain a transcript record for BRCA2 transcript ENST00000380152"
    );

    Ok(())
}

#[tokio::test]
async fn gencode_transcript_fasta_contains_brca2() -> Result<(), AnyError> {
    let client = gencode_client()?;
    let url = gencode_transcript_fasta_url();
    let fasta = fetch_and_gunzip_to_string(&client, &url).await?;

    assert!(
        fasta.lines().any(|line| line.starts_with('>')),
        "Expected GENCODE transcript FASTA to contain at least one header"
    );
    let brca2_header = fasta
        .lines()
        .find(|line| line.starts_with(">ENST00000380152"))
        .expect("Expected transcript FASTA to contain ENST00000380152* header line");
    assert!(
        brca2_header.to_ascii_uppercase().contains("BRCA2"),
        "Expected ENST00000380152 header to mention BRCA2 when present, got '{}'",
        brca2_header
    );

    let seqs = parse_fasta_sequences(&fasta);

    let mut found_brca2_tx = None;
    for (id, seq) in &seqs {
        if id.starts_with("ENST00000380152") {
            found_brca2_tx = Some(seq.as_str());
            break;
        }
    }

    let seq =
        found_brca2_tx.expect("Expected transcript FASTA to contain ENST00000380152* sequence");

    assert!(
        !seq.trim().is_empty(),
        "Expected non-empty BRCA2 transcript sequence"
    );
    assert!(
        is_dna_sequence(seq.trim()),
        "Expected BRCA2 transcript sequence to be DNA, got leading snippet '{}'",
        seq.chars().take(40).collect::<String>()
    );
    assert!(
        seq.len() > 1000,
        "BRCA2 transcript should be long; expected length > 1000 nt, got {}",
        seq.len()
    );

    Ok(())
}

#[tokio::test]
async fn gencode_protein_fasta_basic_sanity() -> Result<(), AnyError> {
    let client = gencode_client()?;
    let url = gencode_protein_fasta_url();
    let fasta = fetch_and_gunzip_to_string(&client, &url).await?;

    assert!(
        fasta.lines().any(|line| line.starts_with('>')),
        "Expected GENCODE protein FASTA to contain at least one header"
    );

    let seqs = parse_fasta_sequences(&fasta);
    assert!(
        !seqs.is_empty(),
        "Expected at least one protein sequence in GENCODE protein FASTA"
    );
    assert!(
        seqs.iter().any(|(id, _)| id.starts_with("ENSP")),
        "Expected at least some GENCODE protein FASTA ids to begin with 'ENSP'"
    );

    let (first_id, first_seq) = &seqs[0];
    assert!(
        !first_seq.trim().is_empty(),
        "Expected first protein sequence '{}' to be non-empty",
        first_id
    );
    assert!(
        is_protein_sequence(first_seq.trim()),
        "Expected protein sequence alphabet for '{}', got leading snippet '{}'",
        first_id,
        first_seq.chars().take(40).collect::<String>()
    );

    Ok(())
}

#[tokio::test]
async fn gencode_invalid_url_returns_error() -> Result<(), AnyError> {
    let client = gencode_client()?;
    let url = format!(
        "{}/{}/nonexistent_file.gtf.gz",
        GENCODE_BASE, GENCODE_RELEASE
    );

    let result = fetch_and_gunzip_to_string(&client, &url).await;
    assert!(
        result.is_err(),
        "Expected fetch_and_gunzip_to_string to return error for invalid GENCODE URL '{}'",
        url
    );

    Ok(())
}
