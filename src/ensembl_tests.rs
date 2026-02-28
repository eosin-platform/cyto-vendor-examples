use anyhow::{Result, ensure};

use crate::ensembl::EnsemblClient;
use crate::ncbi::parse_fasta;

#[tokio::test]
async fn fetch_ensembl_brca2_gene() -> Result<()> {
    let client = EnsemblClient::new()?;
    let gene_id = "ENSG00000139618";
    let gene = client.fetch_gene(gene_id).await?;

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
async fn fetch_ensembl_brca2_transcript() -> Result<()> {
    let client = EnsemblClient::new()?;
    let tx_id = "ENST00000380152";
    let tx = client.fetch_transcript(tx_id).await?;

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
async fn fetch_ensembl_brca2_variation() -> Result<()> {
    let client = EnsemblClient::new()?;
    let species = "homo_sapiens";
    let rs_id = "rs80359273";
    let var = client.fetch_variation(species, rs_id).await?;

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

// NOTE: assembly info / protein / sequence types and fetchers come from `crate::ensembl` now.

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
async fn fetch_ensembl_human_assembly_info() -> Result<()> {
    let client = EnsemblClient::new()?;
    let species = "homo_sapiens";
    let info = client.fetch_assembly_info(species).await?;

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
async fn fetch_ensembl_brca2_protein() -> Result<()> {
    let client = EnsemblClient::new()?;
    let protein_id = "ENSP00000419060";
    let protein = client.fetch_protein(protein_id).await?;

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
async fn fetch_ensembl_brca2_gene_sequence() -> Result<()> {
    let client = EnsemblClient::new()?;
    let gene_id = "ENSG00000139618";
    let seq = client.fetch_sequence(gene_id).await?;

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
async fn fetch_ensembl_brca2_fasta() -> Result<()> {
    let client = EnsemblClient::new()?;
    let gene_id = "ENSG00000139618";

    // Mirror the NCBI FASTA test pattern: fetch metadata, then fetch FASTA.
    let gene = client.fetch_gene(gene_id).await?;
    assert!(
        gene.display_name.eq_ignore_ascii_case("BRCA2"),
        "Expected Ensembl display_name 'BRCA2', got '{}'",
        gene.display_name
    );

    let fasta = client.fetch_fasta(gene_id).await?;
    let (header, sequence) = parse_fasta(&fasta)?;
    ensure!(
        header.contains(gene_id),
        "Unexpected FASTA header (expected it to mention {}), got '{header}'",
        gene_id
    );
    ensure!(
        !sequence.trim().is_empty(),
        "Expected non-empty FASTA sequence for Ensembl gene '{gene_id}'"
    );
    ensure!(
        is_dna_sequence(sequence.trim()),
        "Expected DNA FASTA sequence for gene '{}', got leading snippet '{}'",
        gene_id,
        sequence.chars().take(40).collect::<String>()
    );

    // Cross-check: FASTA payload should match the plain-text sequence endpoint.
    let plain = client.fetch_sequence(gene_id).await?;
    let plain = plain.trim();
    let fasta_seq = sequence.trim();
    ensure!(
        fasta_seq.len() == plain.len(),
        "Ensembl FASTA sequence length mismatch for '{gene_id}': plain={}, fasta={}",
        plain.len(),
        fasta_seq.len()
    );
    ensure!(
        fasta_seq.eq_ignore_ascii_case(plain),
        "Ensembl FASTA content mismatch for '{gene_id}'"
    );

    Ok(())
}

#[tokio::test]
async fn fetch_ensembl_brca2_transcript_sequence() -> Result<()> {
    let client = EnsemblClient::new()?;
    let tx_id = "ENST00000380152";
    let seq = client.fetch_sequence(tx_id).await?;

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
async fn fetch_ensembl_brca2_protein_sequence() -> Result<()> {
    let client = EnsemblClient::new()?;
    let protein_id = "ENSP00000419060";
    let seq = client.fetch_sequence(protein_id).await?;

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
