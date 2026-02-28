use anyhow::{Context, Result};
use cyto_vendor_examples::ncbi::NcbiClient;

#[tokio::test]
async fn fetch_brca2_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;

    let listing = client
        .list_genes("BRCA2[Gene Name] AND 9606[Taxonomy ID]", 0, 20)
        .await?;
    assert!(
        listing.items.contains(&675),
        "Expected NCBI gene listing to include BRCA2 gene ID 675, got {:?}",
        listing.items
    );

    let metadata = client.fetch_gene(675).await?;

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
async fn fetch_brca2_nuccore_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_nuccore_metadata("NM_000059.4").await?;

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
async fn fetch_brca2_protein_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_protein_metadata("NP_000050.3").await?;

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
async fn fetch_human_assembly_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_assembly_metadata("11968211").await?;

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
async fn fetch_example_sra_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_sra_metadata("7426").await?;

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
async fn fetch_example_pubmed_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_pubmed_metadata("31452104").await?;

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
async fn fetch_human_taxonomy_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_taxonomy_metadata("9606").await?;

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
async fn fetch_example_clinvar_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_clinvar_metadata("17875").await?;

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
async fn fetch_example_biosample_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_biosample_metadata("2876").await?;

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
async fn fetch_example_bioproject_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_bioproject_metadata("31213").await?;

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
async fn fetch_human_brca2_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let genes = client
        .list_genes("BRCA2[Gene Name] AND Homo sapiens[Organism]", 0, 3)
        .await
        .context("Failed to list genes")?;
    let gene = client
        .fetch_gene(genes.items[0])
        .await
        .context("Failed to fetch gene metadata")?;
    assert_eq!(gene.symbol, "BRCA2");
    assert_eq!(gene.description, "BRCA2 DNA repair associated");
    assert!(gene.summary.len() > 0, "Expected non-empty BRCA2 summary");
    assert_eq!(gene.chromosome, "13");
    assert_eq!(gene.organism.common_name, "human");
    assert_eq!(gene.organism.scientific_name, "Homo sapiens");
    Ok(())
}

#[tokio::test]
async fn fetch_mesh_breast_neoplasms_from_ncbi() -> Result<()> {
    let client = NcbiClient::new()?;
    let metadata = client.fetch_mesh_metadata("68001943").await?;

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
