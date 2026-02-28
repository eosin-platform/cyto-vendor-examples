//! Integration-style example that fetches BRCA2 metadata from NCBI,
//! resolves a genomic interval, and fetches the corresponding FASTA.
use anyhow::{Context, Result, ensure};

use crate::ncbi::parse_fasta;

#[tokio::main]
async fn main() -> Result<()> {
    println!("Fetching BRCA2 metadata from NCBI...");
    let client = ncbi::NcbiClient::new()?;
    let genes = client
        .list_genes("BRCA2[Gene Name] AND Homo sapiens[Organism]", 0, 3)
        .await
        .context("Failed to list genes")?;
    println!("Genes: {genes:#?}");
    let gene = client
        .fetch_gene(genes.items[0])
        .await
        .context("Failed to fetch gene metadata")?;
    println!("Gene metadata: {gene:#?}");

    let location = gene
        .location_hist
        .first()
        .context("Gene metadata did not contain any locations")?;
    ensure!(
        location.chr_stop >= location.chr_start,
        "Invalid location coordinates: start {} > stop {}",
        location.chr_start,
        location.chr_stop
    );
    println!("fetching");
    let fasta = client
        .fetch_nuccore_fasta_region(
            &location.chr_acc_ver,
            location.chr_start,
            location.chr_stop,
            ncbi::Strand::Forward,
        )
        .await
        .with_context(|| {
            format!(
                "Failed to fetch FASTA for {}:{}-{}",
                location.chr_acc_ver, location.chr_start, location.chr_stop
            )
        })?;
    let (header, sequence) = parse_fasta(&fasta)?;
    ensure!(
        header.contains(&location.chr_acc_ver),
        "Unexpected FASTA header (expected it to mention {}), got '{header}'",
        location.chr_acc_ver
    );
    let expected_len = location.chr_stop - location.chr_start + 1;
    let observed_len = sequence.len() as u64;
    ensure!(
        observed_len == expected_len,
        "FASTA sequence length mismatch for {}:{}-{}: expected {expected_len}, got {observed_len}",
        location.chr_acc_ver,
        location.chr_start,
        location.chr_stop
    );
    println!(
        "Fetched FASTA ok: {}:{}-{} ({} bp)",
        location.chr_acc_ver, location.chr_start, location.chr_stop, observed_len
    );
    Ok(())
}

#[cfg(test)]
mod ensembl;
mod ncbi;

#[cfg(test)]
mod ncbi_tests;

#[cfg(test)]
mod ensembl_tests;

#[cfg(test)]
mod uniprot_test;

#[cfg(test)]
mod gencode_test;

#[cfg(test)]
mod obo_test;

#[cfg(test)]
mod gnomad_test;

#[cfg(test)]
mod ucsc_test;

#[cfg(test)]
mod rcsb_test;

#[cfg(test)]
mod ena_test;

#[cfg(test)]
mod gdc_test;

#[cfg(test)]
mod dbsnp_test;

#[cfg(test)]
mod dbvar_test;

#[cfg(test)]
mod go_test;

#[cfg(test)]
mod hpo_test;

#[cfg(test)]
mod cptac_test;

#[cfg(test)]
mod gtex_test;
