use anyhow::{Result, ensure};

use crate::rcsb::RcsbClient;

#[allow(non_snake_case)]
#[tokio::test]
async fn fetch_rcsb_mmCIF_basic_structure() -> Result<()> {
    let client = RcsbClient::new()?;
    let pdb_id = "6VXX";
    let cif_bytes = client.download_cif_bytes(pdb_id).await?;
    let cif = String::from_utf8_lossy(&cif_bytes);

    assert!(
        !cif.trim().is_empty(),
        "Expected mmCIF response to be non-empty for PDB id '{pdb_id}'"
    );

    let first_non_empty_line = cif
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    assert!(
        first_non_empty_line
            .to_ascii_lowercase()
            .starts_with("data_"),
        "Expected mmCIF first non-empty line to start with 'data_' (case-insensitive) for '{pdb_id}', got '{first_non_empty_line}'"
    );

    assert!(
        first_non_empty_line
            .to_ascii_lowercase()
            .contains(&pdb_id.to_ascii_lowercase()),
        "Expected mmCIF data block name to include PDB id '{pdb_id}' in first line '{first_non_empty_line}'"
    );

    assert!(
        cif.lines()
            .any(|line| line.trim_start().starts_with("loop_")),
        "Expected mmCIF content for '{pdb_id}' to contain at least one 'loop_' line"
    );

    assert!(
        cif.contains("_atom_site.label_atom_id")
            || cif.contains("_entity_poly.pdbx_seq_one_letter_code"),
        "Expected mmCIF content for '{pdb_id}' to include key tags '_atom_site.label_atom_id' or '_entity_poly.pdbx_seq_one_letter_code'"
    );

    assert!(
        cif_bytes.len() > 10 * 1024,
        "Expected mmCIF content for '{pdb_id}' to exceed 10KB, got {} bytes",
        cif_bytes.len()
    );

    Ok(())
}

#[tokio::test]
async fn fetch_rcsb_pdb_basic_structure() -> Result<()> {
    let client = RcsbClient::new()?;
    let pdb_id = "4HHB";
    let pdb = client.download_pdb_text(pdb_id).await?;

    assert!(
        !pdb.trim().is_empty(),
        "Expected PDB response to be non-empty for PDB id '{pdb_id}'"
    );

    let first_non_empty_line = pdb
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or_default();
    assert!(
        first_non_empty_line.starts_with("HEADER") || first_non_empty_line.starts_with("TITLE"),
        "Expected PDB first non-empty line to start with 'HEADER' or 'TITLE' for '{pdb_id}', got '{first_non_empty_line}'"
    );

    assert!(
        pdb.contains("ATOM") || pdb.contains("HETATM"),
        "Expected PDB content for '{pdb_id}' to include 'ATOM' or 'HETATM'"
    );

    let atom_count = pdb
        .lines()
        .filter(|line| line.starts_with("ATOM") || line.starts_with("HETATM"))
        .take(10)
        .count();
    assert!(
        atom_count > 0,
        "Expected PDB content for '{pdb_id}' to contain at least one line starting with 'ATOM' or 'HETATM'"
    );

    assert!(
        pdb.lines()
            .any(|line| line.starts_with("END") || line.starts_with("ENDMDL")),
        "Expected PDB content for '{pdb_id}' to contain an 'END' or 'ENDMDL' line"
    );

    assert!(
        pdb.len() > 10 * 1024,
        "Expected PDB content for '{pdb_id}' to exceed 10KB, got {} bytes",
        pdb.len()
    );

    Ok(())
}

#[tokio::test]
async fn fetch_rcsb_graphql_metadata() -> Result<()> {
    let client = RcsbClient::new()?;
    let pdb_id = "1A2B";
    let entry = client.fetch_entry_metadata(pdb_id).await?;

    assert_eq!(
        entry.rcsb_id.to_ascii_uppercase(),
        pdb_id,
        "Expected RCSB GraphQL entry_id to match requested id '{pdb_id}', got '{}'",
        entry.rcsb_id
    );

    let title = entry
        .r#struct
        .as_ref()
        .map(|s| s.title.trim())
        .unwrap_or_default();
    assert!(
        !title.is_empty(),
        "Expected RCSB GraphQL title to be non-empty for entry '{pdb_id}'"
    );

    let molecular_weight = entry
        .rcsb_entry_info
        .as_ref()
        .and_then(|info| info.molecular_weight)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Expected molecular_weight field in RCSB GraphQL payload for entry '{pdb_id}'"
            )
        })?;
    assert!(
        molecular_weight > 0.0,
        "Expected molecular_weight > 0 for entry '{pdb_id}', got {molecular_weight}"
    );

    Ok(())
}

#[tokio::test]
async fn search_rcsb_entries_text_by_id() -> Result<()> {
    let client = RcsbClient::new()?;
    let pdb_id = "4HHB";

    let result = client.search_entries_text(pdb_id, 0, 10).await?;
    ensure!(
        result
            .items
            .iter()
            .any(|id| id.eq_ignore_ascii_case(pdb_id)),
        "Expected text search for '{pdb_id}' to include the entry id in results, got {:?}",
        result.items
    );

    Ok(())
}

#[tokio::test]
async fn rcsb_invalid_id_returns_error() {
    let client = RcsbClient::new().unwrap();
    let bad_id = "NOTREAL";

    let cif_result = client.download_cif_bytes(bad_id).await;
    assert!(
        cif_result.is_err(),
        "Expected mmCIF fetch to fail for invalid RCSB id '{bad_id}'"
    );
    if let Err(error) = cif_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad_id) || msg.contains("404") || msg.contains("Not Found"),
            "Expected mmCIF error to mention invalid id or 404-like status for '{bad_id}', got '{msg}'"
        );
    }

    let pdb_result = client.download_pdb_text(bad_id).await;
    assert!(
        pdb_result.is_err(),
        "Expected PDB fetch to fail for invalid RCSB id '{bad_id}'"
    );
    if let Err(error) = pdb_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad_id) || msg.contains("404") || msg.contains("Not Found"),
            "Expected PDB error to mention invalid id or 404-like status for '{bad_id}', got '{msg}'"
        );
    }

    let gql_result = client.fetch_entry_metadata(bad_id).await;
    assert!(
        gql_result.is_err(),
        "Expected GraphQL metadata fetch to fail for invalid RCSB id '{bad_id}'"
    );
    if let Err(error) = gql_result {
        let msg = error.to_string();
        assert!(
            msg.contains(bad_id) || msg.contains("404") || msg.contains("Not Found"),
            "Expected GraphQL error to mention invalid id or 404-like status for '{bad_id}', got '{msg}'"
        );
    }
}
