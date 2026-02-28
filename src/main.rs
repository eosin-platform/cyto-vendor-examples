//! Integration-style tests that fetch BRCA2 metadata from NCBI and
//! assert that key fields are parsed correctly.
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
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
    Ok(())
}

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
